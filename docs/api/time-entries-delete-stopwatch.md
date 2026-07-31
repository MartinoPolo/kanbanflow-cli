# Delete Stopwatch entries

> Source: https://kanbanflow.com/api-docs/time-entries-delete-stopwatch
> Scraped: 2026-07-30

# API Documentation

* * *

## Delete Stopwatch entry

### Description

Deletes a Stopwatch entry and updates time spent on related tasks accordingly.

### Permissions

To delete time entries the API token needs the **Update tasks** permission

### Request format

curl -X DELETE https://kanbanflow.com/api/v1/stopwatch-entries/<STOPWATCH\_ENTRY\_ID>

### Example request

curl -X DELETE https://kanbanflow.com/api/v1/stopwatch-entries/EyS84FdLk
