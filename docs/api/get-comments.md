# Get comments

> Source: https://kanbanflow.com/api-docs/get-comments
> Scraped: 2026-07-30

# API Documentation

* * *

## Get comments

### Description

Returns all comments for a task.

### Request format

curl https://kanbanflow.com/api/v1/tasks/<TASK\_ID>/comments

### Example response

\[
    {
        "\_id": "C3VJU6yeb",
        "text": "The report looks good.",
        "createdTimestamp": "2023-12-12T09:34:07Z",
        "authorUserId": "UHJ9JgtA"
    },
    {
        "\_id": "Cks4VwyRB",
        "text": "Thanks!",
        "createdTimestamp": "2023-12-12T12:10:00Z",
        "authorUserId": "UfSTW3ny"
    }
\]
