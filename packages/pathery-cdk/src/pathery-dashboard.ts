import { Construct } from "constructs";
import {
  Column,
  Dashboard,
  GraphWidget,
  LogQueryWidget,
  MathExpression,
  Row,
  Shading,
  TextWidget,
} from "aws-cdk-lib/aws-cloudwatch";
import { RustFunction } from "./rust-function";
import { Duration, Stack } from "aws-cdk-lib";

export interface PatheryDashboardProps {
  indexWriterWorker: RustFunction;
}

export class PatheryDashboard extends Construct {
  constructor(scope: Construct, id: string, props: PatheryDashboardProps) {
    super(scope, id);

    const dashboard = new Dashboard(this, "Resource", {
      dashboardName: `Pathery-${id}-2`,
    });

    let functions = Stack.of(this)
      .node.findAll()
      .filter((c): c is RustFunction => c instanceof RustFunction);

    let successRate = new MathExpression({
      expression: "100 - ((errors / invocations) * 100)",
      period: Duration.minutes(1),
      usingMetrics: {
        errors: props.indexWriterWorker.metricErrors({
          statistic: "sum",
        }),
        invocations: props.indexWriterWorker.metricInvocations({
          statistic: "sum",
        }),
      },
      color: "#72bf6a",
      label: "Success Rate",
    });

    dashboard.addWidgets(
      new LogQueryWidget({
        title: "Errors",
        logGroupNames: functions.map((f) => f.logGroup.logGroupName),
        queryLines: [
          "fields @timestamp, @log, fields.message",
          "filter level = 'ERROR'",
        ],
        width: 24,
      }),
      new Column(
        new TextWidget({
          markdown: "# IndexWriterWorker",
          width: 24,
          height: 1,
        }),
        new Row(
          new GraphWidget({
            liveData: true,
            title: "IndexWriterWorker Execution",
            width: 12,
            left: [
              props.indexWriterWorker.metricDuration({
                period: Duration.minutes(1),
                statistic: "max",
              }),
            ],
            leftYAxis: {
              min: 0,
              label: "Latency (ms)",
              showUnits: false,
            },
            right: [successRate],
            rightYAxis: {
              min: 0,
              max: 100,
              label: "Success Rate (%)",
              showUnits: false,
            },
            leftAnnotations: [
              {
                value:
                  (props.indexWriterWorker.timeout?.toMilliseconds() ?? 3000) *
                  0.75,
                fill: Shading.ABOVE,
                color: "#e6b400",
                label: "Timeout Warning",
              },
              {
                value:
                  props.indexWriterWorker.timeout?.toMilliseconds() ?? 3000,
                fill: Shading.ABOVE,
                color: "#f44336",
                label: "Timeout",
              },
            ],
          })
        )
      )
    );
  }
}
