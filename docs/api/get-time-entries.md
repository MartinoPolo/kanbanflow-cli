# Get time entries

> Source: https://kanbanflow.com/api-docs/get-time-entries
> Scraped: 2026-07-30

# API Documentation

* * *

## Get time entries by task ID

### Description

Returns all time entries (Pomodoro, Stopwatch and manual) for a task. Since Pomodoro and Stopwatch entries can span multiple tasks, they have a partIndex property.

### Request format

curl https://kanbanflow.com/api/v1/tasks/<TASK\_ID>/time-entries

### Example request

curl https://kanbanflow.com/api/v1/tasks/T3s6UGyzY/time-entries

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
        "entryId": "EyS84FdLk",
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
