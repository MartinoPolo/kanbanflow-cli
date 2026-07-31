# Get attachments

> Source: https://kanbanflow.com/api-docs/get-attachments
> Scraped: 2026-07-30

# API Documentation

* * *

## Get attachments

### Description

Returns all attachments for a task.

There are 5 providers: KanbanFlow, Dropbox, Google Drive, OneDrive and Box. Each attachment has a link where you can access the file. For attachments stored at KanbanFlow, there is an additional timestamp for when the link will expire.

### Request format

curl https://kanbanflow.com/api/v1/tasks/<TASK\_ID>/attachments

### Example response

\[
    {
        "\_id": "Aks4VwyRB",
        "provider": "KanbanFlow",
        "name": "report.pdf",
        "size": 12438,
        "mimeType": "application/pdf",
        "link": "https://s3-eu-west-1.amazonaws.com/kanbanflow/...",
        "linkExpiresTimestamp": "2023-02-16T12:42:22.089Z",
        "createdTimestamp": "2023-02-15T12:42:21.673Z",
        "createdByFullName": "John Doe"
    },
    {
        "\_id": "A3VJU6yeb",
        "provider": "Dropbox",
        "name": "chart.png",
        "size": 9712,
        "mimeType": "image/png",
        "link": "https://www.dropbox.com/s/...",
        "createdTimestamp": "2023-02-15T12:43:16.425Z",
        "createdByFullName": "Mary Anderson"
    }
\]
