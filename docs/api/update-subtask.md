# Update subtask

> Source: https://kanbanflow.com/api-docs/update-subtask
> Scraped: 2026-07-30

# API Documentation

* * *

## Update subtask

### Description

Updates a subtask by index or name. Index is 0-based. Name is case-sensitive. Only supply the properties you want to change.

### Request format (index)

curl -X POST https://kanbanflow.com/api/v1/tasks/<TASK\_ID>/subtasks/by-index/<INDEX> -H "Content-type: application/json" -d '{ "<PROPERTY\_1>": <PROPERTY\_VALUE\_1>, "<PROPERTY\_2>": <PROPERTY\_VALUE\_2>, ... "<PROPERTY\_N>": <PROPERTY\_VALUE\_N> }'

### Request format (name)

curl -X POST https://kanbanflow.com/api/v1/tasks/<TASK\_ID>/subtasks/by-name/<NAME> -H "Content-type: application/json" -d '{ "<PROPERTY\_1>": <PROPERTY\_VALUE\_1>, "<PROPERTY\_2>": <PROPERTY\_VALUE\_2>, ... "<PROPERTY\_N>": <PROPERTY\_VALUE\_N> }'

### Example request (index)

curl -X POST https://kanbanflow.com/api/v1/tasks/T3s6UGyzY/subtasks/by-index/2 -H "Content-type: application/json"
 -d '{ "name": "Proofread" }'

### Example request (name)

curl -X POST https://kanbanflow.com/api/v1/tasks/T3s6UGyzY/subtasks/by-name/Proofread -H "Content-type: application/json"
 -d '{ "finished": true }'

### Valid properties

| Property | Type | Comment |
| --- | --- | --- |
| name | String | The name of the subtask |
| finished | Boolean | Use true to indicate if the subtask should be checked. |
| userId | String | Optional. Used for assigning a user to the subtask. If you want to clear the field use **null** as value without any quotes. |
| dueDateTimestamp | String | Optional. The UTC timestamp when the subtask is due. If you want to clear the field use **null** as value without any quotes. |
| dueDateTimestampLocal | String | Optional. Defaults to dueTimestamp if not given. The user's local timestamp when the subtask is due. Used for email reminders. If you want to clear the field use **null** as value without any quotes. |
