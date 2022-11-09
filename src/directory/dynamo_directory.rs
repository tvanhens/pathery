use anyhow::Result;
use aws_sdk_dynamodb::{
    model::AttributeValue::{self, B, N, S},
    output::{PutItemOutput, QueryOutput},
    types::Blob,
};
use std::{cell::Cell, collections::HashMap, io::BufWriter, io::Write, sync::Arc};
use tantivy::{
    directory::{error::OpenReadError, OwnedBytes},
    Directory,
};
use tantivy_common::TerminatingWrite;
use tokio::runtime::Runtime;

trait DDBSave {
    fn serialize(&self) -> HashMap<String, AttributeValue>;

    fn save(&self, table: &DynamoTable) -> Result<()> {
        table.put_item_sync(self.serialize())?;
        Ok(())
    }
}

#[derive(Debug)]
struct DynamoTable {
    table_name: String,
    client: aws_sdk_dynamodb::Client,
    rt: Runtime,
}

impl DynamoTable {
    fn new(table_name: String) -> Result<DynamoTable> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        let config = rt.block_on(aws_config::load_from_env());

        Ok(DynamoTable {
            table_name,
            client: aws_sdk_dynamodb::Client::new(&config),
            rt,
        })
    }

    async fn put_item(&self, item: HashMap<String, AttributeValue>) -> Result<PutItemOutput> {
        Ok(self
            .client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .send()
            .await?)
    }

    fn put_item_sync(&self, item: HashMap<String, AttributeValue>) -> Result<PutItemOutput> {
        self.rt.block_on(self.put_item(item))
    }

    async fn list(&self, pk: &str) -> Result<QueryOutput> {
        Ok(self
            .client
            .query()
            .table_name(&self.table_name)
            .key_condition_expression("#pk = :pk")
            .expression_attribute_names("#pk", "pk")
            .expression_attribute_values(":pk", S(pk.to_string()))
            .send()
            .await?)
    }

    fn list_sync(&self, pk: &str) -> Result<QueryOutput> {
        self.rt.block_on(self.list(pk))
    }
}

struct DynamoFilePart {
    directory_id: String,
    path: String,
    part: usize,
    content: Vec<u8>,
}

impl DDBSave for DynamoFilePart {
    fn serialize(&self) -> HashMap<String, AttributeValue> {
        HashMap::from([
            (
                "pk".to_string(),
                S(format!(
                    "directory|{}|file|{}",
                    self.directory_id, self.path
                )),
            ),
            ("sk".to_string(), S(format!("part|{}", self.part))),
            ("directory".to_string(), S(self.directory_id.to_string())),
            ("path".to_string(), S(self.path.to_string())),
            ("part".to_string(), N(self.part.to_string())),
            ("content".to_string(), B(Blob::new(self.content.clone()))),
        ])
    }
}

struct DynamoFileHeader {
    directory_id: String,
    path: String,
}

impl DDBSave for DynamoFileHeader {
    fn serialize(&self) -> HashMap<String, AttributeValue> {
        HashMap::from([
            (
                "pk".to_string(),
                S(format!("directory|{}|file-headers", self.directory_id)),
            ),
            ("sk".to_string(), S(format!("header|{}", self.path))),
            ("path".to_string(), S(self.path.to_string())),
            ("directory".to_string(), S(self.directory_id.to_string())),
        ])
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
        let results = self
            .table
            .list_sync(&format!(
                "directory|{}|file|{}",
                self.id,
                path.to_str().unwrap()
            ))
            .map_err(|e| OpenReadError::IoError {
                io_error: std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
                filepath: path.to_path_buf(),
            })?;

        let parts = results
            .items()
            .unwrap()
            .iter()
            .map(|x| DynamoFilePart {
                content: x
                    .get("content")
                    .unwrap()
                    .as_b()
                    .unwrap()
                    .clone()
                    .into_inner(),
                directory_id: x.get("directory").unwrap().as_s().unwrap().to_string(),
                part: x
                    .get("part")
                    .unwrap()
                    .as_n()
                    .unwrap()
                    .parse::<usize>()
                    .unwrap(),
                path: x.get("directory").unwrap().as_s().unwrap().to_string(),
            })
            .collect::<Vec<DynamoFilePart>>();

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
    use super::Directory;
    use super::DynamoDirectory;
    use super::TerminatingWrite;
    use super::Write;

    #[test]
    fn write_and_read() {
        let path = std::path::Path::new("hello.txt");
        let part1 = "hello world".as_bytes();
        let part2 = "stuff and things".as_bytes();

        let directory = DynamoDirectory::create("test-table", "1234").unwrap();
        let mut writer = directory.open_write(path).unwrap();

        writer.get_mut().write(part1).unwrap();
        writer.get_mut().write(part2).unwrap();

        writer.terminate().unwrap();

        let handle = directory.open_read(path).unwrap();

        let read_bytes = handle.read_bytes().unwrap().to_vec();

        assert_eq!(read_bytes, [part1, part2].concat().to_vec());
    }
}
