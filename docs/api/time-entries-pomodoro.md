# Get Pomodoro entries

> Source: https://kanbanflow.com/api-docs/time-entries-pomodoro
> Scraped: 2026-07-30

# API Documentation

* * *

## Get Pomodoro entry by ID

### Description

Returns a single Pomodoro entry

### Permissions

To get time entries the API token needs the **Read tasks** permission

### Request format

curl https://kanbanflow.com/api/v1/pomodoro-entries/<POMODORO\_ENTRY\_ID>

### Example request

curl https://kanbanflow.com/api/v1/pomodoro-entries/EyS84FdLk

### Example response

{
    "\_id": "EyS84FdLk",
    "userId": "UHJ9JgtA",
    "targetWorkMinutes": 25,
    "success": false,
    "interruptionReason": "Colleague interrupted",
    "startTimestamp": "2023-01-02T08:30:00Z",
    "parts": \[
        {
            "endTimestamp": "2023-01-02T08:45:00Z",
            "taskId": "T3s6UGyzY",
            "boardId": "BZrWANX8T"
        },
        {
            "endTimestamp": "2023-01-02T08:51:00Z",
            "taskId": "TZ8avKBVt",
            "boardId": "BZrWANX8T"
        }
    \]
}

### Response properties

| Property | Type | Comment |
| --- | --- | --- |
| \_id | String | The ID of the Pomodoro entry. |
| userId | String | The ID of the user that added the Pomodoro entry. |
| targetWorkMinutes | Integer | The user defined duration in minutes of a complete Pomodoro. |
| success | Boolean | True if Pomodoro points were given for this session. |
| interruptionReason | String | If Pomodoro was interrupted this is the reason the user gave. |
| startTimestamp | String | The UTC timestamp when the Pomodoro entry started, for example 2023-12-31T09:00:00Z. |
| parts | Array | Refers to parts of the Pomodoro session. Each part includes task ID, board ID and end timestamp.  
The end timestamp is a UTC timestamp, for example 2023-12-31T17:00:00Z. |
| comment | String | A comment to the time entry. Only included when set. |
| labelNames | Array | Labels for the time entry. Only included when there are any items. Example: \["Billable", "Project X"\]. |

## Get Pomodoro entries for board

### Description

Returns all Pomodoro time entries for the board belonging to the token filtered by a time period.

When filtering on a _from-to_ timespan all entries whose _start-end_ timespan intersects the filter timespan will be included.

### Request format

curl https://kanbanflow.com/api/v1/pomodoro-entries?from=<TIMESTAMP>&to=<TIMESTAMP>&limit=<INTEGER>&userId=<USER\_ID>

### Example request

curl https://kanbanflow.com/api/v1/pomodoro-entries?from=2023-01-01T00:00:00Z&to=2023-01-03T00:00:00Z&limit=100

### Example response

\[
    {
        "\_id": "EyS84FdLk",
        "userId": "UHJ9JgtA",
        "targetWorkMinutes": 25,
        "success": false,
        "interruptionReason": "Colleague interrupted",
        "startTimestamp": "2023-01-02T08:30:00Z",
        "parts": \[
            {
                "endTimestamp": "2023-01-02T08:45:00Z",
                "taskId": "T3s6UGyzY",
                "boardId": "BZrWANX8T"
            },
            {
                "endTimestamp": "2023-01-02T08:51:00Z",
                "taskId": "T3s6UGyzY",
                "boardId": "BZrWANX8T"
            }
        \]
    },
    {
        "\_id": "Eks4VwyRB",
        "userId": "UHJ9JgtA",
        "targetWorkMinutes": 15,
        "success": true,
        "startTimestamp": "2023-01-02T09:00:00Z",
        "parts": \[
            {
                "endTimestamp": "2023-01-02T09:10:00Z",
                "taskId": "T3s6UGyzY",
                "boardId": "BZrWANX8T"
            },
            {
                "endTimestamp": "2023-01-02T09:15:00Z",
                "taskId": "T3s6UGyzY",
                "boardId": "BZrWANX8T"
            }
        \]
    },
    {
        "\_id": "E3VJU6yeb",
        "userId": "UHJ9JgtA",
        "targetWorkMinutes": 10,
        "success": true,
        "startTimestamp": "2023-01-02T09:20:00Z",
        "parts": \[
            {
                "endTimestamp": "2023-01-02T09:30:00Z",
                "taskId": "T3s6UGyzY",
                "boardId": "BZrWANX8T"
            }
        \]
    }
\]

### Valid parameters

| Parameter | Type | Comment |
| --- | --- | --- |
| from | String | Either _from_ or _to_ is required. If you want to give a timestamp for 1 January 2023 05:30 (UTC), you can either use 2023-01-01T05:30Z or 1672551000000 (time since epoch). |
| to | String | Either _from_ or _to_ is required. If you want to give a timestamp for 1 January 2023 05:30 (UTC), you can either use 2023-01-01T05:30Z or 1672551000000 (time since epoch). |
| userId | String | Optional. Filter entries by user ID. |
| limit | Integer | Optional. The highest value that can be used is 1000. Default: 100. |
