# Delete manual entries

> Source: https://kanbanflow.com/api-docs/time-entries-delete-manual
> Scraped: 2026-07-30

# API Documentation

* * *

## Delete manual time entry

### Description

Deletes a manual time entry and updates time spent on related task accordingly.

### Permissions

To delete time entries the API token needs the **Update tasks** permission

### Request format

curl -X DELETE https://kanbanflow.com/api/v1/manual-time-entries/<MANUAL\_TIME\_ENTRY\_ID>

### Example request

curl -X DELETE https://kanbanflow.com/api/v1/manual-time-entries/EyS84FdLk
