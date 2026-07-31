# Delete Pomodoro entries

> Source: https://kanbanflow.com/api-docs/time-entries-delete-pomodoro
> Scraped: 2026-07-30

# API Documentation

* * *

## Delete Pomodoro entry

### Description

Deletes a Pomodoro entry and updates time spent on related tasks accordingly.

### Permissions

To delete time entries the API token needs the **Update tasks** permission

### Request format

curl -X DELETE https://kanbanflow.com/api/v1/pomodoro-entries/<POMODORO\_ENTRY\_ID>

### Example request

curl -X DELETE https://kanbanflow.com/api/v1/pomodoro-entries/EyS84FdLk
