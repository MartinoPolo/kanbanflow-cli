# Update task

> Source: https://kanbanflow.com/api-docs/update-task
> Scraped: 2026-07-30

# API Documentation

* * *

## Update task

### Description

Update an existing task. You only supply the properties that you want to change.

### Request format

curl -X POST https://kanbanflow.com/api/v1/tasks/<TASK\_ID> -H "Content-type: application/json" -d '{ "<PROPERTY\_1>": <PROPERTY\_VALUE\_1>, "<PROPERTY\_2>": <PROPERTY\_VALUE\_2>, ... "<PROPERTY\_N>": <PROPERTY\_VALUE\_N> }'

### Example request

curl -X POST https://kanbanflow.com/api/v1/tasks/T3s6UGyzY -H "Content-type: application/json" -d '{ "color": "green", "columnId": "C9LIn5sEEpqT" }'

### Valid properties

| Property | Type | Comment |
| --- | --- | --- |
| name | String | The name of the task. |
| columnId | String | The ID of the column. See **Get board** tab for information about how to get a column's ID. |
| columnIndex | Integer | _Deprecated_. Use **columnId** instead. 0 is the first column, 1 the second column, etc. |
| swimlaneId | String | The ID of the swimlane, if any. See **Get board** for information about how to get a swimlane's ID. |
| position | String / Integer | The index of the task in the column/swimlane or date group. Valid values: Positive number, "top" or "bottom". **Note:** The index is zero-based. |
| color | String | Valid color values: yellow, white, red, green, blue, purple, orange, cyan, brown, or magenta. It needs to be in lowercase. |
| description | String | The description of the task. |
| number | Object | The task number consists of an integer value and an optional prefix. If you want to clear the field use **null** as value without any quotes. Examples: { "value": 5 }, { "prefix": "BUG-", "value": 5 } |
| responsibleUserId | String | The ID of the responsible user. See **Get users** tab for information about how to get a user's ID. If you want to clear the field use **null** as value without any quotes. |
| totalSecondsSpent | Integer | _Deprecated_. Examples: 1 hour = 3600, 2 hours = 7200 etc. **Note:** Changes that lessen the property are **ignored**. Changes increasing the property will create _manual time entries_ to reflect the added time. See **Add manual time entry** under **Create task** for more control of how time is added to the task. |
| totalSecondsEstimate | Integer | Examples: 1 hour = 3600, 2 hours = 7200 etc. |
| pointsEstimate | Float | Estimated points for the task. |
| groupingDate | String | Can only be used if column is date grouped. Valid format is YYYY-MM-DD, e.g. 2023-12-31. Use null or empty string if you want it grouped as unknown date. If not set and moving task to a date grouped column, then it will automatically be set to today's date for the UTC timezone. |
| dates | Array | A date requires a dueTimestamp and a targetColumnId. See **Create/update date** for more details about the optional properties. Example: { "dueTimestamp": "2023-03-01T12:00:00Z", "targetColumnId": "COxkPjd0wra4" } |
| subTasks | Array | A subtask consists of a name and an optional finished flag. See **Create subtask** for more details. Examples: { "name": "Write", "finished": true }, { "name": "Proofread" } |
| labels | Array | A label consists of a name and an optional pinned flag. See **Create label** for more details. Examples: { "name": "Project X" }, { "name": "Priority", "pinned": true } |
| collaborators | Array | A collaborator consists of a userId. See **Add collaborator** for more details. Examples: { "userId": "UHJ9JgtA" } |
| customFields | Array | The custom fields set on the task. See **Create/update custom fields** for details. Examples: { "customFieldId": "g3aB3m", "value": { "text": "London" } }, { "customFieldId": "91yRUs", "value": { "number": 123.45 } } |
| timeline | Object | A timeline consists of a start and an end date. Valid format is YYYY-MM-DD, e.g. 2023-12-31. If you want to clear the field use **null** as value without any quotes. Example: { "start": "2024-01-01", "end": "2024-01-31" } |
