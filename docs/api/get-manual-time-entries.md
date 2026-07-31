# Get manual time entries

> Source: https://kanbanflow.com/api-docs/get-manual-time-entries
> Scraped: 2026-07-30

# API Documentation

* * *

## Get manual time entry by ID

### Description

Returns a single manual time entry from a task.

### Request format

curl https://kanbanflow.com/api/v1/tasks/<TASK\_ID>/manual-time-entries/<MANUAL\_TIME\_ENTRY\_ID>

### Example request

curl https://kanbanflow.com/api/v1/tasks/T3s6UGyzY/manual-time-entries/EyS84FdLk

### Example response

{
    "\_id": "EyS84FdLk",
    "createdTimestamp": "2023-01-02T12:50:00Z",
    "startTimestamp": "2023-01-02T08:30:00Z",
    "endTimestamp": "2023-01-02T12:00:00Z",
    "userId": "UHJ9JgtA",
    "taskId": "T3s6UGyzY"
}

### Response properties

| Property | Type | Comment |
| --- | --- | --- |
| \_id | String | The ID of the manual time entry. |
| startTimestamp | String | The UTC timestamp when the manual time entry started, for example 2023-12-31T09:00:00Z. |
| endTimestamp | String | The UTC timestamp when the manual time entry ended, for example 2023-12-31T17:00:00Z. |
| userId | String | The ID of the user that added the manual time entry. |
| taskId | String | The ID of the task that the manual time entry was created on. |
| comment | String | A comment to the time entry. Only included when set. |
| labelNames | Array | Labels for the time entry. Only included when there are any items. Example: \["Billable", "Project X"\]. |

## Get all manual time entries

### Description

Returns all manual time entries for a task.

See **Get manual time entry by ID** for details of included properties for each individual manual time entry.

### Request format

curl https://kanbanflow.com/api/v1/tasks/<TASK\_ID>/manual-time-entries

### Example request

curl https://kanbanflow.com/api/v1/tasks/T3s6UGyzY/manual-time-entries

### Example response

\[
    {
        "\_id": "EyS84FdLk",
        "createdTimestamp": "2023-01-02T12:50:00Z",
        "startTimestamp": "2023-01-02T08:30:00Z",
        "endTimestamp": "2023-01-02T12:00:00Z",
        "userId": "UHJ9JgtA",
        "taskId": "T3s6UGyzY"
    },
    {
        "\_id": "Eks4VwyRB",
        "createdTimestamp": "2023-01-02T17:10:00Z",
        "startTimestamp": "2023-01-02T13:00:00Z",
        "endTimestamp": "2023-01-02T17:00:00Z",
        "userId": "UHJ9JgtA",
        "taskId": "T3s6UGyzY"
    }
\]
