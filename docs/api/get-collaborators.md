# Get collaborators

> Source: https://kanbanflow.com/api-docs/get-collaborators
> Scraped: 2026-07-30

# API Documentation

* * *

## Get collaborators

### Description

Returns all collaborators for a task.

### Request format

curl https://kanbanflow.com/api/v1/tasks/<TASK\_ID>/collaborators

### Example request

curl https://kanbanflow.com/api/v1/tasks/T3s6UGyzY/collaborators

### Example response

\[
    {
        "userId": "UHJ9JgtA"
    },
    {
        "userId": "UfSTW3ny"
    }
\]
