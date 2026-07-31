# Get manual entries

> Source: https://kanbanflow.com/api-docs/time-entries-manual
> Scraped: 2026-07-30

# API Documentation

* * *

## Get manual time entry by ID

### Description

Returns a single manual time entry

### Permissions

To get time entries the API token needs the **Read tasks** permission

### Request format

curl https://kanbanflow.com/api/v1/manual-time-entries/<MANUAL\_TIME\_ENTRY\_ID>

### Example request

curl https://kanbanflow.com/api/v1/manual-time-entries/EyS84FdLk

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
| createdTimestamp | String | The UTC timestamp when the manual time entry was created, for example 2023-12-31T20:00:00Z. |
| startTimestamp | String | The UTC timestamp when the manual time entry started, for example 2023-12-31T09:00:00Z. |
| endTimestamp | String | The UTC timestamp when the manual time entry ended, for example 2023-12-31T17:00:00Z. |
| userId | String | The ID of the user that added the manual time entry. |
| taskId | String | The ID of the task that the manual time entry was created on. |
| comment | String | A comment to the time entry. Only included when set. |
| labelNames | Array | Labels for the time entry. Only included when there are any items. Example: \["Billable", "Project X"\]. |

## Get manual time entries for board

### Description

Returns all manual time entries for the board belonging to the token filtered by a time period.

When filtering on a _from-to_ timespan all entries whose _start-end_ timespan intersects the filter timespan will be included.

### Request format

curl https://kanbanflow.com/api/v1/manual-time-entries?from=<TIMESTAMP>&to=<TIMESTAMP>&limit=<INTEGER>&userId=<USER\_ID>

### Example request

curl https://kanbanflow.com/api/v1/manual-time-entries?from=2023-01-01T00:00:00Z&to=2023-01-03T00:00:00Z&limit=100

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
    },
    {
        "\_id": "E3VJU6yeb",
        "createdTimestamp": "2023-01-03T12:50:00Z",
        "startTimestamp": "2023-01-03T08:30:00Z",
        "endTimestamp": "2023-01-03T12:00:00Z",
        "userId": "UHJ9JgtA",
        "taskId": "T3s6UGyzY"
    }
\]

### Valid parameters

| Parameter | Type | Comment |
| --- | --- | --- |
| from | String | Either _from_ or _to_ is required. If you want to give a timestamp for 1 January 2023 05:30 (UTC), you can either use 2023-01-01T05:30Z or 1672551000000 (time since epoch). |
| to | String | Either _from_ or _to_ is required. If you want to give a timestamp for 1 January 2023 05:30 (UTC), you can either use 2023-01-01T05:30Z or 1672551000000 (time since epoch). |
| userId | String | Optional. Filter entries by user ID. |
| limit | Integer | Optional. The highest value that can be used is 1000. Default: 100. |
