# Delete collaborator

> Source: https://kanbanflow.com/api-docs/delete-collaborator
> Scraped: 2026-07-30

# API Documentation

* * *

## Delete collaborator

### Description

Removes a collaborator from a task.

### Request format

curl -X DELETE https://kanbanflow.com/api/v1/tasks/<TASK\_ID>/collaborators/by-user-id/<USER\_ID>

### Example request

curl -X DELETE https://kanbanflow.com/api/v1/tasks/T3s6UGyzY/collaborators/by-user-id/UHJ9JgtA
