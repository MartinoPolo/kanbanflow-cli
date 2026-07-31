# Events

> Source: https://kanbanflow.com/api-docs/events
> Scraped: 2026-07-30

# API Documentation

* * *

## Board events

### Description

Returns all events on the board, optionally filtered by a time period. If you want to give a timestamp for 1 January 2023 05:30 (UTC), you can either use _2023-01-01T05:30Z_ or _1672551000000_ (time since epoch). A maximum of 100 events are returned by default.

### Request format

curl https://kanbanflow.com/api/v1/board/events?from=<TIMESTAMP>&to=<TIMESTAMP>

### Example request (get events between two timestamps)

curl "https://kanbanflow.com/api/v1/board/events?from=2023-01-01T00:00Z&to=2023-01-02T00:00Z"

### Example request (get the first event)

curl "https://kanbanflow.com/api/v1/board/events?limit=1"

### Example request (get the last event)

curl "https://kanbanflow.com/api/v1/board/events?limit=1&order=descending"

### Example response

{
    "\_id": "EB8eEYnSp",
    "timestamp": "2023-01-01T17:45:21Z",
    "userId": "UHJ9JgtA",
    "detailedEvents": \[
        {
            "eventType": "taskChanged",
            "taskId": "T3s6UGyzY",
            "changedProperties": \[
                {
                    "property": "color",
                    "oldValue": "blue",
                    "newValue": "red"
                }
            \]
        }
    \]
}

### Valid parameters

| Parameter | Type | Comment |
| --- | --- | --- |
| from | String | Optional. If you want to give a timestamp for 1 January 2023 05:30 (UTC), you can either use 2023-01-01T05:30Z or 1672551000000 (time since epoch). |
| to | String | Optional. If you want to give a timestamp for 1 January 2023 05:30 (UTC), you can either use 2023-01-01T05:30Z or 1672551000000 (time since epoch). |
| limit | Integer | Optional. The highest value that can be used is 100. Default: 100. |
| order | String | Optional. "ascending" or "descending". Default: "ascending", except when only "to" is given (without "from") where it defaults to "descending". |

## Task events

### Description

Returns all events for a task, optionally filtered by a time period. If you want to give a timestamp for 1 January 2023 05:30 (UTC), you can either use _2023-01-01T05:30Z_ or _1672551000000_ (time since epoch). A maximum of 100 events are returned by default.

### Request format

curl https://kanbanflow.com/api/v1/tasks/<TASK\_ID>/events?from=<TIMESTAMP>&to=<TIMESTAMP>

### Example request (get events from a timestamp)

curl "https://kanbanflow.com/api/v1/tasks/T3s6UGyzY/events?from=2023-01-01T00:00Z"

### Example request (get the first event)

curl "https://kanbanflow.com/api/v1/tasks/T3s6UGyzY/events?limit=1"

### Example request (get the last event)

curl "https://kanbanflow.com/api/v1/tasks/T3s6UGyzY/events?limit=1&order=descending"

### Example response

{
    "\_id": "EB8eEYnSp",
    "timestamp": "2023-01-01T17:45:21Z",
    "userId": "UHJ9JgtA",
    "detailedEvents": \[
        {
            "eventType": "taskChanged",
            "taskId": "T3s6UGyzY",
            "changedProperties": \[
                {
                    "property": "color",
                    "oldValue": "blue",
                    "newValue": "red"
                }
            \]
        }
    \]
}

### Valid parameters

| Parameter | Type | Comment |
| --- | --- | --- |
| from | String | Optional. If you want to give a timestamp for 1 January 2023 05:30 (UTC), you can either use 2023-01-01T05:30Z or 1672551000000 (time since epoch). |
| to | String | Optional. If you want to give a timestamp for 1 January 2023 05:30 (UTC), you can either use 2023-01-01T05:30Z or 1672551000000 (time since epoch). |
| limit | Integer | Optional. The highest value that can be used is 100. Default: 100. |
| order | String | Optional. "ascending" or "descending". Default: "ascending", except when only "to" is given (without "from") where it defaults to "descending". |
