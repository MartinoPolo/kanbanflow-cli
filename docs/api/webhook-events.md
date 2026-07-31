# Events

> Source: https://kanbanflow.com/api-docs/webhook-events
> Scraped: 2026-07-30

# API Documentation

* * *

## Webhook events

Each event will deliver specific details relevant to the event type.

### Task created example

{
    "eventType": "taskCreated",
    "userId": "UHJ9JgtA",
    "userFullName": "John Doe",
    "timestamp": "2023-10-20T14:45:06.331Z",
    "task": {
        "\_id": "T3s6UGyzY",
        "name": "Write report",
        "description": "For school",
        "color": "green",
        "columnId": "C9LIn5sEEpqT",
        "totalSecondsSpent": 0,
        "totalSecondsEstimate": 0,
        "swimlaneId": "SqEB6zxG41e2",
        "dates": \[
            {
                "targetColumnId": "COxkPjd0wra4",
                "status": "active",
                "dateType": "dueDate",
                "dueTimestamp": "2023-10-20T15:00:00Z",
                "dueTimestampLocal": "2023-10-20T17:00:00+02:00"
            }
        \],
        "subTasks": \[
            {
                "name": "Proofread",
                "finished": false
            }
        \],
        "labels": \[
            {
                "name": "Writing",
                "pinned": false
            }
        \]
    }
}

### Task changed example

{
    "eventType": "taskChanged",
    "userId": "UHJ9JgtA",
    "userFullName": "John Doe",
    "timestamp": "2023-10-20T14:45:13.775Z",
    "task": {
        "\_id": "T3s6UGyzY",
        "name": "Write report",
        "description": "For school",
        "color": "blue",
        "columnId": "C9LIn5sEEpqT",
        "totalSecondsSpent": 0,
        "totalSecondsEstimate": 0,
        "swimlaneId": "SqEB6zxG41e2",
        "dates": \[
            {
                "targetColumnId": "COxkPjd0wra4",
                "status": "active",
                "dateType": "dueDate",
                "dueTimestamp": "2023-10-20T15:00:00Z",
                "dueTimestampLocal": "2023-10-20T17:00:00+02:00"
            }
        \],
        "subTasks": \[
            {
                "name": "Proofread",
                "finished": false
            }
        \],
        "labels": \[
            {
                "name": "Writing",
                "pinned": false
            }
        \]
    },
    "changedProperties": \[
        {
            "property": "color",
            "oldValue": "red",
            "newValue": "blue"
        }
    \]
}

### Task deleted example

{
    "eventType": "taskDeleted",
    "userId": "UHJ9JgtA",
    "userFullName": "John Doe",
    "timestamp": "2023-10-20T14:45:26.138Z",
    "taskId": "T3s6UGyzY",
    "taskName": "Write report"
}

### Comment created example

{
    "eventType": "taskCommentCreated",
    "userId": "UHJ9JgtA",
    "userFullName": "John Doe",
    "timestamp": "2023-10-20T14:45:26.138Z",
    "taskId": "T3s6UGyzY",
    "taskName": "Write report",
    "taskComment": {
        "\_id": "C3VJU6yeb",
        "text": "Finished writing report.",
        "authorUserId": "UHJ9JgtA",
        "createdTimestamp": "2023-10-20T14:45:26"
    }
}

### Comment changed example

{
    "eventType": "taskCommentChanged",
    "userId": "UHJ9JgtA",
    "userFullName": "John Doe",
    "timestamp": "2023-10-21T14:45:26.138Z",
    "taskId": "T3s6UGyzY",
    "taskName": "Write report",
    "taskComment": {
        "\_id": "C3VJU6yeb",
        "text": "Finished writing report.",
        "authorUserId": "UHJ9JgtA",
        "createdTimestamp": "2023-10-20T14:45:26",
        "updatedTimestamp": "2023-10-21T14:45:26"
    },
    "oldTaskCommentText": "Finished writing report.",
    "newTaskCommentText": "Almost finished writing report."
}

### Comment deleted example

{
    "eventType": "taskCommentDeleted",
    "userId": "UHJ9JgtA",
    "userFullName": "John Doe",
    "timestamp": "2023-10-20T14:45:26.138Z",
    "taskId": "T3s6UGyzY",
    "taskName": "Write report",
    "taskCommentId": "C3VJU6yeb",
    "taskCommentText": "Finished writing report."
}
