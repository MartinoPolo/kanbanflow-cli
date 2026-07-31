# Create webhook

> Source: https://kanbanflow.com/api-docs/create-webhook
> Scraped: 2026-07-30

# API Documentation

* * *

## Create webhook

### Description

Create a new webhook.

### Request format

curl -X POST https://kanbanflow.com/api/v1/webhooks -H "Content-type: application/json" -d '{ "<PROPERTY\_1>": <PROPERTY\_VALUE\_1>, "<PROPERTY\_2>": <PROPERTY\_VALUE\_2>, ... "<PROPERTY\_N>": <PROPERTY\_VALUE\_N> }'

### Example request

curl -X POST https://kanbanflow.com/api/v1/webhooks -H "Content-type: application/json" -d '{ "name": "My hook", "callbackUrl": "https://myserver.com/mycallback", "events": \[{ "name": "taskCreated" }\] }'

### Example response

{
    "webhookId": "Wh45gJV2mDbEv",
    "secret": "nfnY7Q2i8kJRteWsopSKVc9wkE"
}

### Valid properties

| Property | Type | Comment |
| --- | --- | --- |
| name | String | Optional. |
| callbackUrl | String | Required. Must return status code 200 on an HTTP HEAD request. Only supports http(s):// protocols. |
| events | Array | Required. Valid event names: taskCreated, taskChanged, taskDeleted, taskCommentCreated, taskCommentChanged, taskCommentDeleted. At least one must be given.  
Example: \[{ "name": "taskCreated" }, { "name": "taskChanged" }, { "name": "taskCommentCreated" }\] |
| filter | Object | Optional. Filter on a specific column, swimlane and/or changes to certain properties.  
Examples: { "columnId": "C9LIn5sEEpqT" }, { "swimlaneId": "SqEB6zxG41e2" }, { "changedProperties": \["color", "totalSecondsSpent"\] } |
