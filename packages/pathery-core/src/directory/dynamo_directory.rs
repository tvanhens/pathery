use anyhow::Result;
use aws_sdk_dynamodb::{
    model::AttributeValue::{self, B, N, S},
    types::Blob,
};
use std::{cell::Cell, collections::HashMap, io::BufWriter, io::Write, sync::Arc};
use tantivy::{directory::OwnedBytes, Directory};
use tantivy_common::TerminatingWrite;

use crate::dynamo::{DynamoRecord, DynamoTable};

struct DynamoFilePart {
    directory_id: String,
    path: String,
    part: usize,
    content: Vec<u8>,
}

impl DynamoRecord<(&str, &str)> for DynamoFilePart {
    fn format_pk(pk: (&str, &str)) -> String {
        format!("directory|{}|file|{}", pk.0, pk.1)
    }

    fn serialize(&self) -> HashMap<String, AttributeValue> {
        HashMap::from([
            (
                "pk".to_string(),
                S(Self::format_pk((&self.directory_id, &self.path))),
            ),
            ("sk".to_string(), S(format!("part|{}", self.part))),
            ("directory".to_string(), S(self.directory_id.to_string())),
            ("path".to_string(), S(self.path.to_string())),
            ("part".to_string(), N(self.part.to_string())),
            ("content".to_string(), B(Blob::new(self.content.clone()))),
        ])
    }

    fn deserialize(item: &HashMap<String, AttributeValue>) -> Self {
        DynamoFilePart {
            content: item
                .get("content")
                .unwrap()
                .as_b()
                .unwrap()
                .as_ref()
                .to_vec(),
            directory_id: item.get("directory").unwrap().as_s().unwrap().to_string(),
            part: item
                .get("part")
                .unwrap()
                .as_n()
                .unwrap()
                .parse::<usize>()
                .unwrap(),
            path: item.get("directory").unwrap().as_s().unwrap().to_string(),
        }
    }
}

struct DynamoFileHeader {
    directory_id: String,
    path: String,
}

impl DynamoRecord<&str> for DynamoFileHeader {
    fn format_pk(pk: &str) -> String {
        format!("directory|{}|file-headers", pk)
    }

    fn serialize(&self) -> HashMap<String, AttributeValue> {
        HashMap::from([
            ("pk".to_string(), S(Self::format_pk(&self.directory_id))),
            ("sk".to_string(), S(format!("header|{}", self.path))),
            ("path".to_string(), S(self.path.to_string())),
            ("directory".to_string(), S(self.directory_id.to_string())),
        ])
    }

    fn deserialize(item: &HashMap<String, AttributeValue>) -> Self {
        DynamoFileHeader {
            directory_id: item.get("directory").unwrap().as_s().unwrap().to_string(),
            path: item.get("directory").unwrap().as_s().unwrap().to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DynamoDirectory {
    table: Arc<DynamoTable>,
    id: String,
}

impl DynamoDirectory {
    pub fn create(table_name: &str, id: &str) -> Result<DynamoDirectory> {
        Ok(DynamoDirectory {
            table: Arc::new(DynamoTable::new(table_name.to_string())?),
            id: id.to_string(),
        })
    }
}

impl Directory for DynamoDirectory {
    fn get_file_handle(
        &self,
        path: &std::path::Path,
    ) -> Result<Box<dyn tantivy::directory::FileHandle>, tantivy::directory::error::OpenReadError>
    {
        let parts = DynamoFilePart::list(&self.table, (&self.id, path.to_str().unwrap())).unwrap();

        let content: Vec<u8> = parts.iter().flat_map(|part| part.content.clone()).collect();

        Ok(Box::new(OwnedBytes::new(content)))
    }

    fn delete(
        &self,
        _path: &std::path::Path,
    ) -> Result<(), tantivy::directory::error::DeleteError> {
        todo!()
    }

    fn exists(
        &self,
        _path: &std::path::Path,
    ) -> Result<bool, tantivy::directory::error::OpenReadError> {
        todo!()
    }

    fn open_write(
        &self,
        path: &std::path::Path,
    ) -> Result<tantivy::directory::WritePtr, tantivy::directory::error::OpenWriteError> {
        struct VirtualFile {
            path: String,
            table: Arc<DynamoTable>,
            directory_id: String,
            current_part: Cell<usize>,
            part_buffer: Vec<DynamoFilePart>,
        }

        impl Write for VirtualFile {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                let part = self.current_part.get();
                self.part_buffer.push(DynamoFilePart {
                    directory_id: self.directory_id.to_string(),
                    path: self.path.clone(),
                    part,
                    content: buf.to_vec(),
                });
                self.current_part.set(part + 1);
                Ok(buf.len())
            }

            fn flush(&mut self) -> std::io::Result<()> {
                loop {
                    if let Some(part) = self.part_buffer.pop() {
                        part.save(&self.table).map_err(|e| {
                            std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
                        })?;
                    } else {
                        break;
                    }
                }

                let header = DynamoFileHeader {
                    directory_id: self.directory_id.to_string(),
                    path: self.path.to_string(),
                };

                header
                    .save(&self.table)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

                Ok(())
            }
        }

        impl TerminatingWrite for VirtualFile {
            fn terminate_ref(&mut self, _: tantivy_common::AntiCallToken) -> std::io::Result<()> {
                self.flush()
            }
        }

        Ok(BufWriter::new(Box::new(VirtualFile {
            table: self.table.clone(),
            directory_id: self.id.clone(),
            part_buffer: Vec::new(),
            path: path.to_str().unwrap().to_string(),
            current_part: Cell::new(0),
        })))
    }

    fn atomic_read(
        &self,
        _path: &std::path::Path,
    ) -> Result<Vec<u8>, tantivy::directory::error::OpenReadError> {
        todo!()
    }

    fn atomic_write(&self, _path: &std::path::Path, _data: &[u8]) -> std::io::Result<()> {
        todo!()
    }

    fn sync_directory(&self) -> std::io::Result<()> {
        todo!()
    }

    fn watch(
        &self,
        _watch_callback: tantivy::directory::WatchCallback,
    ) -> tantivy::Result<tantivy::directory::WatchHandle> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use config::Config;
    use serde::Deserialize;
    use serde::Serialize;

    use super::Directory;
    use super::DynamoDirectory;
    use super::TerminatingWrite;
    use super::Write;

    #[derive(Serialize, Deserialize, Debug)]
    struct DevStackConfig {
        #[serde(rename = "TestTableName")]
        test_table_name: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct DevEnvOutputs {
        #[serde(rename = "pathery-dev")]
        pathery_dev: DevStackConfig,
    }

    fn load_config() -> DevEnvOutputs {
        let config = Config::builder()
            .add_source(config::File::with_name(
                "node_modules/@internal/dev-env/cdk-outputs.json",
            ))
            .build()
            .unwrap();

        config.try_deserialize::<DevEnvOutputs>().unwrap()
    }

    #[test]
    fn write_and_read() {
        let config = load_config();
        println!("{:?}", config);
        let path = std::path::Path::new("hello.txt");
        let part1 = "hello world".as_bytes();
        let part2 = "stuff and things".as_bytes();

        let directory =
            DynamoDirectory::create(&config.pathery_dev.test_table_name, "1234").unwrap();
        let mut writer = directory.open_write(path).unwrap();

        writer.get_mut().write(part1).unwrap();
        writer.get_mut().write(part2).unwrap();

        writer.terminate().unwrap();

        let handle = directory.open_read(path).unwrap();

        let read_bytes = handle.read_bytes().unwrap().to_vec();

        assert_eq!(read_bytes, [part1, part2].concat().to_vec());
    }
}
