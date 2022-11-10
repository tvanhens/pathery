import { App } from "aws-cdk-lib";
import { DevResourcesStack } from "./stack";

const app = new App();

new DevResourcesStack(app, "dev-resources");
