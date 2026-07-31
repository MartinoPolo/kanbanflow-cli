# Move task to another board

> Source: https://kanbanflow.com/api-docs/move-task-to-another-board
> Scraped: 2026-07-30

# API Documentation

* * *

## Move task to another board

### Description

To move a task between boards, you need an API token for both the source and target board.

For the source board, authentication is done as described on the **Authentication** tab. The source API token needs to have the **Delete tasks** permission.

For the target board, you need to send a _X-Target-Authorization_ header in the request. It is constructed in the same way as the regular _Authorization_ header, but with the target board's API token. The target API token needs to have the **Create tasks** permission.

### Request format

curl -X POST https://kanbanflow.com/api/v1/tasks/<TASK\_ID>/move-to-board/<TARGET\_BOARD\_ID> -H "Content-type: application/json"
 -H "X-Target-Authorization: Bearer <SECRET\_TOKEN\_FOR\_TARGET\_BOARD>"
 -d '{ "<PROPERTY\_1>": <PROPERTY\_VALUE\_1>, "<PROPERTY\_2>": <PROPERTY\_VALUE\_2>, ... "<PROPERTY\_N>": <PROPERTY\_VALUE\_N> }'

### Example request

curl -X POST https://kanbanflow.com/api/v1/tasks/T3s6UGyzY/move-to-board/B2fM3pfTU -H "Content-type: application/json" -H "X-Target-Authorization: Bearer 55d24f7baedb75b03d1f168aee6466a7"
 -d '{ "columnId": "CBO1VNGqDc4K" }'

### Valid properties

| Property | Type | Comment |
| --- | --- | --- |
| columnId | String | The ID of the column to move to. If not provided, the task will be moved to the first column. See **Get board** tab for information about how to get a column's ID. |
| swimlaneId | String | The ID of the swimlane to move to, if any. If not provided and the board has swimlanes, the task will be moved to the first swimlane. See **Get board** for information about how to get a swimlane's ID. |
| groupingDate | String | Can only be used if column is date grouped. Valid format is YYYY-MM-DD, e.g. 2023-12-31. If not set and moving task to a date grouped column, then it will automatically be set to today's date for the UTC timezone. |
