# Get labels

> Source: https://kanbanflow.com/api-docs/get-labels
> Scraped: 2026-07-30

# API Documentation

* * *

## Get labels

### Description

Returns all labels for a task.

### Request format

curl https://kanbanflow.com/api/v1/tasks/<TASK\_ID>/labels

### Example request

curl https://kanbanflow.com/api/v1/tasks/T3s6UGyzY/labels

### Example response

\[
    {
        "name": "Priority",
        "pinned": true
    },
    {
        "name": "Project X",
        "pinned": false
    }
\]
