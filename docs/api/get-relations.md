# Get relations

> Source: https://kanbanflow.com/api-docs/get-relations
> Scraped: 2026-07-30

# API Documentation

* * *

## Get all relations

### Description

Returns all relations for a task.

### Request format

curl https://kanbanflow.com/api/v1/tasks/<TASK\_ID>/relations

### Example request

curl https://kanbanflow.com/api/v1/tasks/7HPu4D9Q/relations

### Example response

\[
    {
        "relationType": "requiredBy",
        "relatedTaskId": "QuAzRUx8",
        "relatedTaskName": "Send out newsletter"
    },
    {
        "relationType": "dependsOn",
        "relatedTaskId": "7HPu4D9Q",
        "relatedTaskName": "Plan exhibition",
        "relatedTaskBoardId": "s82koS"
    }
\]

### Response properties

| Property | Type | Comment |
| --- | --- | --- |
| relationType | String | The type of relation to the related task. Can be either _Relates to_ (relatesTo), _Depends on_ (dependsOn) or _Required by_ (requiredBy). |
| relatedTaskId | String | The ID of the related task. |
| relatedTaskName | String | The name of the related task. |
| relatedTaskBoardId | String | The ID for the board where the related task is located. Only included if the related task is not on the same board as the API token. |
