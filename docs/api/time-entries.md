# Time entries

> Source: https://kanbanflow.com/api-docs/time-entries
> Scraped: 2026-07-30

# API Documentation

* * *

## Get time entries for board

### Description

Returns all time entries (Pomodoro, Stopwatch and manual) for the board belonging to the token filtered by a time period. Since Pomodoro and Stopwatch entries can span multiple tasks, they have a partIndex property.

When filtering on a _from-to_ timespan all entries whose _start-end_ timespan intersects the filter timespan will be included.

### Request format

curl https://kanbanflow.com/api/v1/time-entries?from=<TIMESTAMP>&to=<TIMESTAMP>&limit=<INTEGER>&userId=<USER\_ID>

### Example request (from, limit)

curl https://kanbanflow.com/api/v1/time-entries?from=2023-01-01T00:00:00Z&limit=100

### Example request (from, to, userId)

curl https://kanbanflow.com/api/v1/time-entries?from=2023-01-01T00:00:00Z&to=2023-01-03T00:00:00Z&userId=UHJ9JgtA

### Example response

\[
    {
        "entryId": "EyS84FdLk",
        "type": "manual",
        "userId": "UHJ9JgtA",
        "taskId": "KXF2C1oH",
        "startTimestamp": "2023-01-02T08:30:00Z",
        "endTimestamp": "2023-01-02T12:00:00Z"
    },
    {
        "entryId": "Eks4VwyRB",
        "type": "pomodoro",
        "userId": "UHJ9JgtA",
        "taskId": "kX75LPwk",
        "startTimestamp": "2023-01-02T13:00:00Z",
        "endTimestamp": "2023-01-02T13:25:00Z",
        "partIndex": 0
    },
    {
        "entryId": "E3VJU6yeb",
        "type": "stopwatch",
        "userId": "UHJ9JgtA",
        "taskId": "WarF7wJJ",
        "startTimestamp": "2023-01-02T13:00:00Z",
        "endTimestamp": "2023-01-02T17:00:00Z",
        "partIndex": 0
    }
\]

### Response properties

| Property | Type | Comment |
| --- | --- | --- |
| entryId | String | The ID of the original time entry. |
| type | String | Type of time entry. There are three types: "pomodoro", "stopwatch" and "manual". |
| userId | String | The ID of the user that created the time entry. |
| taskId | String | The ID of the task that the time entry was created on. |
| startTimestamp | String | The UTC timestamp when the time entry started, for example 2023-12-31T09:00:00Z. |
| endTimestamp | String | The UTC timestamp when the time entry ended, for example 2023-12-31T17:00:00Z. |
| partIndex | String | The index of the "parts" array in the original Pomodoro or Stopwatch entry. Property is not set on time entries of type "manual". |
| comment | String | A comment to the time entry. Only included when set. |
| labelNames | Array | Labels for the time entry. Only included when there are any items. Example: \["Billable", "Project X"\]. |

### Valid parameters

| Parameter | Type | Comment |
| --- | --- | --- |
| from | String | Either _from_ or _to_ is required. If you want to give a timestamp for 1 January 2023 05:30 (UTC), you can either use 2023-01-01T05:30Z or 1672551000000 (time since epoch). |
| to | String | Either _from_ or _to_ is required. If you want to give a timestamp for 1 January 2023 05:30 (UTC), you can either use 2023-01-01T05:30Z or 1672551000000 (time since epoch). |
| userId | String | Optional. Filter entries by user ID. |
| limit | Integer | Optional. The highest value that can be used is 1000. Default: 100. |
