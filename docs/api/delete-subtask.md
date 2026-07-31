# Delete subtask

> Source: https://kanbanflow.com/api-docs/delete-subtask
> Scraped: 2026-07-30

# API Documentation

* * *

## Delete subtask

### Description

Deletes a subtask by index or name. Index is 0-based. Name is case-sensitive.

### Request format (index)

curl -X DELETE https://kanbanflow.com/api/v1/tasks/<TASK\_ID>/subtasks/by-index/<INDEX>

### Request format (name)

curl -X DELETE https://kanbanflow.com/api/v1/tasks/<TASK\_ID>/subtasks/by-name/<NAME>

### Example request (index)

curl -X DELETE https://kanbanflow.com/api/v1/tasks/T3s6UGyzY/subtasks/by-index/2

### Example request (name)

curl -X DELETE https://kanbanflow.com/api/v1/tasks/T3s6UGyzY/subtasks/by-name/Proofread
