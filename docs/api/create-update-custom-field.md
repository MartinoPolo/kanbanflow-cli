# Create/update custom field

> Source: https://kanbanflow.com/api-docs/create-update-custom-field
> Scraped: 2026-07-30

# API Documentation

* * *

## Create/update custom field value for task

### Description

Create/update custom field value for task. If no value exists for custom field it is added, otherwise it is replaced.

### Request format

curl -X POST https://kanbanflow.com/api/v1/tasks/<TASK\_ID>/custom-fields/<CUSTOM\_FIELD\_ID> -H "Content-type: application/json" -d '{ "<PROPERTY\_1>": <PROPERTY\_VALUE\_1>, "<PROPERTY\_2>": <PROPERTY\_VALUE\_2>, ... "<PROPERTY\_N>": <PROPERTY\_VALUE\_N> }'

### Example request

curl -X POST https://kanbanflow.com/api/v1/tasks/g3aB3m/custom-fields -H "Content-type: application/json"
 -d '{ "value": { "text": "London" } }'

### Valid properties

| Property | Type | Comment |
| --- | --- | --- |
| value | Object | The value to set for the custom field on this task. For _Text_ or _Dropdown_ custom fields it has a single property "text". For _Number_ it has a single property "number". Examples: { "text": "London" }, { "number": 123.45 } |
