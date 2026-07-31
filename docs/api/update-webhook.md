# Update webhook

> Source: https://kanbanflow.com/api-docs/update-webhook
> Scraped: 2026-07-30

# API Documentation

* * *

## Update webhook

### Description

Update an existing webhook. Only supply the properties you want to change.

### Request format

curl -X POST https://kanbanflow.com/api/v1/webhooks/<WEBHOOK\_ID> -H "Content-type: application/json" -d '{ "<PROPERTY\_1>": <PROPERTY\_VALUE\_1>, "<PROPERTY\_2>": <PROPERTY\_VALUE\_2>, ... "<PROPERTY\_N>": <PROPERTY\_VALUE\_N> }'

### Example request

curl -X POST https://kanbanflow.com/api/v1/webhooks/W5hfpsT5 -H "Content-type: application/json" -d '{ "name": "Task changed hook", "events": \[{ "name": "taskChanged" }\] }'

### Valid properties

| Property | Type | Comment |
| --- | --- | --- |
| name | String | The name of the webhook. |
| callbackUrl | String | Must return status code 200 on an HTTP HEAD request. Only supports http(s):// protocols. |
| events | Array | Valid event names: taskCreated, taskChanged, taskDeleted, taskCommentCreated, taskCommentChanged, taskCommentDeleted. At least one must be given.  
Example: \[{ "name": "taskCreated" }, { "name": "taskChanged" }, { "name": "taskCommentCreated" }\] |
| filter | Object | Filter on a specific column, swimlane and/or changes to certain properties.  
Examples: { "columnId": "C9LIn5sEEpqT" }, { "swimlaneId": "SqEB6zxG41e2" }, { "changedProperties": \["color", "totalSecondsSpent"\] } |
