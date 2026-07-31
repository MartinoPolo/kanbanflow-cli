# Get subtasks

> Source: https://kanbanflow.com/api-docs/get-subtasks
> Scraped: 2026-07-30

# API Documentation

* * *

## Get subtasks

### Description

Returns all subtasks for a task.

### Request format

curl https://kanbanflow.com/api/v1/tasks/<TASK\_ID>/subtasks

### Example request

curl https://kanbanflow.com/api/v1/tasks/T3s6UGyzY/subtasks

### Example response

\[
    {
        "name": "Write",
        "finished": true
    },
    {
        "name": "Proofread",
        "finished": false,
        "dueDateTimestamp": "2023-03-01T12:00:00Z",
        "dueDateTimestampLocal": "2023-03-01T13:00:00+01:00"
    },
    {
        "name": "Publish",
        "finished": false,
        "userId": "UHJ9JgtA"
    }
\]
