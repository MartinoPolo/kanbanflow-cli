# Update manual time entry

> Source: https://kanbanflow.com/api-docs/update-manual-time-entry
> Scraped: 2026-07-30

# API Documentation

* * *

## Update manual time entry

### Description

Updates an existing manual time entry. You only supply the properties that you want to change.

### Request format

curl -X POST https://kanbanflow.com/api/v1/tasks/<TASK\_ID>/manual-time-entries/<MANUAL\_TIME\_ENTRY\_ID> -H "Content-type: application/json" -d '{ "<PROPERTY\_1>": <PROPERTY\_VALUE\_1>, "<PROPERTY\_2>": <PROPERTY\_VALUE\_2>, ... "<PROPERTY\_N>": <PROPERTY\_VALUE\_N> }'

### Example request

curl -X POST https://kanbanflow.com/api/v1/tasks/T3s6UGyzY/manual-time-entries/EyS84FdLk -H "Content-type: application/json" -d '{ "startTimestamp": "2023-01-02T10:00:00Z" }'

### Valid properties

| Property | Type | Comment |
| --- | --- | --- |
| startTimestamp | String | The UTC timestamp when the manual time entry started, for example 2023-12-31T09:00:00Z. |
| endTimestamp | String | The UTC timestamp when the manual time entry ended, for example 2023-12-31T17:00:00Z. |
| comment | String | A comment to the time entry. Limited to 50 characters. |
| labelNames | Array | Labels for the time entry. Array of strings. Example: \["Billable", "Project X"\]. |
