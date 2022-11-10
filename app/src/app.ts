import { App } from "aws-cdk-lib";
import { AppStack } from "./stack";

const app = new App();

new AppStack(app, "app-dev");
