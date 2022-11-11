import { App } from "aws-cdk-lib";
import { PatheryStack } from "./pathery-stack";

const app = new App();

new PatheryStack(app, "pathery-dev");
