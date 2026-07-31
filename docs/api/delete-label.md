# Delete label

> Source: https://kanbanflow.com/api-docs/delete-label
> Scraped: 2026-07-30

# API Documentation

* * *

## Delete label

### Description

Deletes a label by name. Name is case-sensitive.

### Request format

curl -X DELETE https://kanbanflow.com/api/v1/tasks/<TASK\_ID>/labels/by-name/<NAME>

### Example request

curl -X DELETE https://kanbanflow.com/api/v1/tasks/T3s6UGyzY/labels/by-name/Priority
