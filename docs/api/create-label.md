# Create label

> Source: https://kanbanflow.com/api-docs/create-label
> Scraped: 2026-07-30

# API Documentation

* * *

## Create label

### Description

Create a label. By default it will be added as the last label of its task, but you can optionally use an _insertIndex_ query parameter to specify where in the label list it should be added.

### Request format

curl -X POST https://kanbanflow.com/api/v1/tasks/<TASK\_ID>/labels -H "Content-type: application/json" -d '{ "<PROPERTY\_1>": <PROPERTY\_VALUE\_1>, "<PROPERTY\_2>": <PROPERTY\_VALUE\_2>, ... "<PROPERTY\_N>": <PROPERTY\_VALUE\_N> }'

### Example request

curl -X POST https://kanbanflow.com/api/v1/tasks/T3s6UGyzY/labels -H "Content-type: application/json"
 -d '{ "name": "Project X" }'

### Example response

{
    "insertIndex": 0
}

### Valid properties

| Property | Type | Comment |
| --- | --- | --- |
| name | String | Required. The name of the label. |
| pinned | Boolean | Optional (default=false). Use true to indicate that the label should be pinned. |
