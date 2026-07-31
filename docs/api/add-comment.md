# Add comment

> Source: https://kanbanflow.com/api-docs/add-comment
> Scraped: 2026-07-30

# API Documentation

* * *

## Add comment

### Description

Adds a comment to a task.

### Request format

curl -X POST https://kanbanflow.com/api/v1/tasks/<TASK\_ID>/comments -H "Content-type: application/json"
 -d '{ "<PROPERTY\_1>": <PROPERTY\_VALUE\_1>, "<PROPERTY\_2>": <PROPERTY\_VALUE\_2>, ... "<PROPERTY\_N>": <PROPERTY\_VALUE\_N> }'

### Example request

curl -X POST https://kanbanflow.com/api/v1/tasks/T3s6UGyzY/comments -H "Content-type: application/json"
 -d '{ "text": "Lorem ipsum dolor sit amet" }'

### Example response

{
    "taskCommentId": "C9LIn5sEEpqT"
}

### Valid properties

| Property | Type | Comment |
| --- | --- | --- |
| text | String | Required. The comment text. |
| authorUserId | String | Optional (default=API user's ID). The ID of the comment author. See **Get users** tab for information about how to get a user's ID. |
| createdTimestamp | String | Optional (default=time now). The UTC timestamp for when the comment was created, for example 2023-12-31T00:00:00.000Z. |
