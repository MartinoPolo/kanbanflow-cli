# Get dates

> Source: https://kanbanflow.com/api-docs/get-dates
> Scraped: 2026-07-30

# API Documentation

* * *

## Get dates

### Description

Returns all dates for a task.

### Request format

curl https://kanbanflow.com/api/v1/tasks/<TASK\_ID>/dates

### Example request

curl https://kanbanflow.com/api/v1/tasks/T3s6UGyzY/dates

### Example response

\[
    {
        "status": "active",
        "dateType": "dueDate",
        "dueTimestamp": "2023-03-01T12:00:00Z",
        "dueTimestampLocal": "2023-03-01T13:00:00+01:00",
        "targetColumnId": "COxkPjd0wra4"
    }
\]
