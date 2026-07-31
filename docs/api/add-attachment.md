# Add attachment

> Source: https://kanbanflow.com/api-docs/add-attachment
> Scraped: 2026-07-30

# API Documentation

* * *

## Add attachment

### Description

Adds an attachment to a task.

Accepts a multipart/form-data request containing the file to attach. The form data parameter must be named "file". The attachment will be given the same name as the filename that is provided for the "file" parameter. E.g. "file=@accounting.xlsx" will add an attachment named "accounting.xlsx".

It is not possible to attach files directly from third party services such as Dropbox, Google Drive, OneDrive or Box. The file contents must be downloaded first and then uploaded as a regular attachment.

**Note:** If using **Curl**, make sure to add an "@" before the file path.

### Request format

curl -F "file=@<FILE\_PATH>" https://kanbanflow.com/api/v1/tasks/<TASK\_ID>/attachments

### Example request

curl -F "file=@report.pdf" https://kanbanflow.com/api/v1/tasks/T3s6UGyzY/attachments

### Example response

{
    "taskAttachmentId": "Aks4VwyRB"
}

### Valid form data parameters

| Parameter | Type | Comment |
| --- | --- | --- |
| file | File | Required. File to upload. |
