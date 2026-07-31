# Create subtask

> Source: https://kanbanflow.com/api-docs/create-subtask
> Scraped: 2026-07-30

# API Documentation

* * *

## Create subtask

### Description

Create a subtask. By default it will be added as the last subtask of its task, but you can optionally use an _insertIndex_ query parameter to specify where in the subtask list it should be added.

### Request format

curl -X POST https://kanbanflow.com/api/v1/tasks/<TASK\_ID>/subtasks -H "Content-type: application/json" -d '{ "<PROPERTY\_1>": <PROPERTY\_VALUE\_1>, "<PROPERTY\_2>": <PROPERTY\_VALUE\_2>, ... "<PROPERTY\_N>": <PROPERTY\_VALUE\_N> }'

### Example request

curl -X POST https://kanbanflow.com/api/v1/tasks/T3s6UGyzY/subtasks -H "Content-type: application/json"
 -d '{ "name": "Proofread" }'

### Example response

{
    "insertIndex": 0
}

### Valid properties

| Property | Type | Comment |
| --- | --- | --- |
| name | String | Required. The name of the subtask. |
| finished | Boolean | Optional (default=false). Use true to indicate if the subtask should be checked. |
| userId | String | Optional. Used for assigning a user to the subtask. |
| dueDateTimestamp | String | Optional. The UTC timestamp when the subtask is due. |
| dueDateTimestampLocal | String | Optional. Defaults to dueTimestamp if not given. The user's local timestamp when the subtask is due. Used for email reminders. |
