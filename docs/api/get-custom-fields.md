# Get custom fields

> Source: https://kanbanflow.com/api-docs/get-custom-fields
> Scraped: 2026-07-30

# API Documentation

* * *

## Get values for all custom fields set on task

### Description

Returns all custom field values for a task.

### Request format

curl https://kanbanflow.com/api/v1/tasks/<TASK\_ID>/custom-fields

### Example request

curl https://kanbanflow.com/api/v1/tasks/7HPu4D9Q/custom-fields

### Example response

\[
    {
        "customFieldId": "g3aB3m",
        "value": {
            "text": "London"
        }
    },
    {
        "customFieldId": "91yRUs",
        "value": {
            "number": 123.45
        }
    }
\]

### Response properties

| Property | Type | Comment |
| --- | --- | --- |
| customFieldId | String | The ID of the custom field. See **Get custom fields** on the top level for details. |
| value | Object | The value for the custom field on this task. For _Text_ or _Dropdown_ custom fields it has a single property "text". For _Number_ it has a single property "number". Examples: { "text": "London" }, { "number": 123.45 } |
