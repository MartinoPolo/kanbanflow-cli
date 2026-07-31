# Update label

> Source: https://kanbanflow.com/api-docs/update-label
> Scraped: 2026-07-30

# API Documentation

* * *

## Update label

### Description

Updates a label by name. Name is case-sensitive. Only supply the properties you want to change.

### Request format

curl -X POST https://kanbanflow.com/api/v1/tasks/<TASK\_ID>/labels/by-name/<NAME> -H "Content-type: application/json" -d '{ "<PROPERTY\_1>": <PROPERTY\_VALUE\_1>, "<PROPERTY\_2>": <PROPERTY\_VALUE\_2>, ... "<PROPERTY\_N>": <PROPERTY\_VALUE\_N> }'

### Example request

curl -X POST https://kanbanflow.com/api/v1/tasks/T3s6UGyzY/labels/by-name/Priority -H "Content-type: application/json"
 -d '{ "pinned": true }'

### Valid properties

| Property | Type | Comment |
| --- | --- | --- |
| name | String | The name of the label |
| pinned | Boolean | Use true to indicate if the label should be pinned. |
