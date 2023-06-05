Kotosiro Sharing Server
==============================

 Kotosiro Sharing is a Rust-based Delta Sharing Server that includes administration functionality.
Unlike [the reference implementation of a Delta Sharing Server](https://github.com/delta-io/delta-sharing),
which primarily focuses on the API specification and uses static file-based sharing information,
Kotosiro Sharing manages its sharing information through an API.

<p align="center">
  <img src="images/workflow.png" width="85%"/>
</p>

Supported Platforms
==============================

| Amazon AWS      | Google GCP       | Microsoft Azure |
|:---------------:|:----------------:|:---------------:|
| :green_square:  | :green_square:   | :red_square:    |

Cofigure Credentials for Cloud Storage Backends
==============================
 1. **Amazon AWS**


 To access the S3 Delta Table backend, you need to create an IAM user with an Amazon S3 permissions policy.
Once you've created the IAM user, you must configure the profile name and region to allow Kotosiro Sharing
Server to access the S3 bucket. The location of the credentials file is specified by the environment variable
`AWS_SHARED_CREDENTIALS_FILE`. If this variable is not set, the credentials file should be located at `~/.aws/credentials`.
 Kotosiro Sharing utilizes the [Object Store](https://crates.io/crates/object_store) crate with the [aws-config](https://crates.io/crates/aws-config)
feature, which requires the `AWS_PROFILE` and `AWS_REGION` environment variables if you use S3 Delta Table backend.
 
  2. **Google GCP**

 To access the GCS Delta Table backend, you need to create a GCS service account. The location of the GCP service
account private key JSON is specified by the environment variable `GOOGLE_APPLICATION_CREDENTIALS`. If this
variable is not set, the private key JSON file should be located at `~/.gcp/service-account-file.json`.

  3. **Microsoft Azure**
  
 Microsoft Azure backed Delta Tables will be supported in the near future.
 
Starting Kotosiro Sharing
==============================

 You can start Kotosiro Sharing using one of the following two options:
 
 1. [Docker Hub](https://hub.docker.com/r/kotosiro/sharing)
 2. [Docker Compose](https://github.com/kotosiro/docker-compose-files)
 
 Please choose the option that best fits your needs and follow the instructions in the corresponding link to start
Kotosiro Sharing. You can also find deployment examples [here](examples).
 
Starting the Development Server
==============================

 Since the implementation is still in the early stages, only the development server is currently available. A Helm
chart will be added to the project in the near future.
 
 To run the development server, execute the following commands in this directory:

```bash
 $ just docker
 $ just server
```

 To run the unit tests, execute the following commands in this directory:

```bash
 $ just docker
 $ just test
 $ just testdb
```
 
Create a New Sharing via the API
==============================

 Once you've started the development server, you can create a new sharing via the API. Follow these steps:
 
 1. Log in to Kotosiro Sharing and get the admin access token by running the following command:
 
```bash
 $ curl -s -X POST http://localhost:8080/admin/login \
        -H "Content-Type: application/json" \
        -d '{"account": "kotosiro", "password": "password"}' \
        | jq '.'
{
  "profile": {
    "shareCredentialsVersion": 1,
    "endpoint": "http://127.0.0.1:8080",
    "bearerToken": "YOUR_ADMIN_ACCESS_TOKEN",
    "expirationTime": "2023-04-09 19:34:04 UTC"
  }
}
```
 
 2. Register a new share by running the following command:
 
```bash
  $ curl -s -X POST "http://localhost:8080/admin/shares" \
         -H "Authorization: Bearer YOUR_ADMIN_ACCESS_TOKEN" \
         -H "Content-Type: application/json" \
         -d'{ "name": "share1", "description": "This is a test share" }' \
         | jq '.'
{
  "share": {
    "id": "6986c361-5e6a-4554-b698-11875d6598e0",
    "name": "share1"
  }
}
```
 
 3. Register a new table by running the following command:

```bash
 $ curl -s -X POST "http://localhost:8080/admin/tables"
        -H "Authorization: Bearer YOUR_ADMIN_ACCESS_TOKEN" \
        -H "Content-Type: application/json" \
        -d'{ "name": "table1", "location": "s3://kotosiro-sharing-test/examination", "description": "This is a test table" }' \
        | jq '.'
{
  "table": {
    "id": "579df9cd-a674-459d-9599-d38d54583cd0",
    "name": "table1",
    "location": "s3://kotosiro-sharing-test/examination"
  }
}
```

 4. Register a new table as a part of schema1 in the share1 by running the following command:
 
```bash
 $ curl -s -X POST "http://localhost:8080/admin/shares/share1/schemas/schema1/tables" \
        -H "Authorization: Bearer YOUR_ADMIN_ACCESS_TOKEN" \
        -H "Content-Type: application/json" \
        -d'{ "table": "table1" }' \
        | jq '.'
{
  "schema": {
    "id": "689ed733-bec8-4796-a2dd-4f82dce6beab",
    "name": "schema1"
  }
}
```

 5. Issue a new recipient profile by running the following command:

```bash
 $ curl -s -X GET "http://localhost:8080/admin/profile" \
        -H "Authorization: Bearer YOUR_ADMIN_ACCESS_TOKEN" \
        -H "Content-Type: application/json" \
        | jq '.'
{
  "profile": {
    "shareCredentialsVersion": 1,
    "endpoint": "http://127.0.0.1:8080",
    "bearerToken": "YOUR_RECIPIENT_ACCESS_TOKEN",
    "expirationTime": "2023-04-09 19:55:19 UTC"
  }
}
```

Kotosiro Sharing Configuration
==============================

 All `TOML`, `JSON`, `YAML`, `INI`, `RON`, and `JSON5` files located in the configuration directory will be loaded as configuration files[^1].
The path to the configuration directory can be set using the `KOTOSIRO_CONF_DIR` environment variable. You can also configure Kotosiro Sharing
using the corresponding environment variables, which is helpful when setting up a Kubernetes cluster[^2]. Please be sure that the environment
variables `AWS_SHARED_CREDENTIALS_FILE` and `GOOGLE_APPLICATION_CREDENTIALS` are set properly if necessary.
Below is a list of the configuration variables:

| Name                 | Environment Variable        | Required | Description                                                                      |
|:--------------------:|:---------------------------:|:--------:|----------------------------------------------------------------------------------|
| `db_url`             | KOTOSIRO_DB_URL             | yes      | URL of PostgreSQL server                                                         |
| `server_addr`        | KOTOSIRO_SERVER_ADDR        | yes      | URL of Kotosiro Sharing server which will be used for sharing profile            |
| `server_bind`        | KOTOSIRO_SERVER_BIND        | yes      | IP address of Korosiro Sharing server which will be used for Axum server binding |
| `admin_name`         | KOTOSIRO_ADMIN_NAME         | yes      | Default admin user name                                                          |
| `admin_email`        | KOTOSIRO_ADMIN_EMAIL        | yes      | Default admin user email                                                         |
| `admin_password`     | KOTOSIRO_ADMIN_PASSWORD     | yes      | Default admin user password                                                      |
| `admin_namespace`    | KOTOSIRO_ADMIN_NAMESPACE    | yes      | Default admin user namespace                                                     |
| `admin_ttl`          | KOTOSIRO_ADMIN_TTL          | yes      | Default admin user access token TTL in seconds                                   |
| `signed_url_ttl`     | KOTOSIRO_SIGNED_URL_TTL     | yes      | Valid duration of signed URL of cloud backends in seconds                        |
| `jwt_secret`         | KOTOSIRO_JWT_SECRET         | yes      | JWT secret key                                                                   |
| `use_json_log`       | KOTOSIRO_USE_JSON_LOG       | yes      | If this value set to be true, log outputs in JSON format                         |
| `log_filter`         | KOTOSIRO_LOG_FILTER         | yes      | Tracing log filter                                                               |

[^1]: An example configuration can also be found at [`config`](https://github.com/kotosiro/sharing/tree/main/config) directory.

[^2]: When Kotosiro Sharing detects duplicated configuration variables, the values from environment variables take precedence over those from configuration files.

[^3]: These variables may be required when you use the corresponding cloud backends. If these variables are not set when dealing with tables located in the corresponding storage backends, Kotosito server will return unsigned URLs instead and may cause internal server errors.

API
==============================

[SEE ALSO](https://github.com/delta-io/delta-sharing/blob/main/PROTOCOL.md)

| Status             | Official       | Method | URL                                                                |
|:------------------:|:--------------:|--------|--------------------------------------------------------------------|
| :heavy_check_mark: | :red_square:   | GET    | */swagger-ui*                                                      |
| :heavy_check_mark: | :red_square:   | POST   | */admin/login*                                                     |
| :heavy_check_mark: | :red_square:   | GET    | */admin/profile*                                                   |
| :heavy_check_mark: | :red_square:   | GET    | */admin/accounts*                                                  |
| :heavy_check_mark: | :red_square:   | POST   | */admin/accounts*                                                  |
| :heavy_check_mark: | :red_square:   | GET    | */admin/accounts/{account}*                                        |
| :heavy_check_mark: | :red_square:   | POST   | */admin/shares*                                                    |
| :heavy_check_mark: | :red_square:   | GET    | */admin/tables*                                                    |
| :heavy_check_mark: | :red_square:   | POST   | */admin/tables*                                                    |
| :heavy_check_mark: | :red_square:   | GET    | */admin/tables/{table}*                                            |
| :heavy_check_mark: | :red_square:   | POST   | */admin/shares/{share}/schemas/{schema}/tables*                    |
|                    | :red_square:   | POST   | */admin/shares/{share}/all-tables*                                 |
| :heavy_check_mark: | :green_square: | GET    | */shares*                                                          |
| :heavy_check_mark: | :green_square: | GET    | */shares/{share}*                                                  |
| :heavy_check_mark: | :green_square: | GET    | */shares/{share}/schemas*                                          |
| :heavy_check_mark: | :green_square: | GET    | */shares/{share}/schemas/{schema}/tables*                          |
| :heavy_check_mark: | :green_square: | GET    | */shares/{share}/all-tables*                                       |
| :heavy_check_mark: | :green_square: | GET    | */shares/{share}/schemas/{schema}/tables/{table}/version*          |
| :heavy_check_mark: | :green_square: | GET    | */shares/{share}/schemas/{schema}/tables/{table}/metadata*         |
| :heavy_check_mark: | :green_square: | POST   | */shares/{share}/schemas/{schema}/tables/{table}/query*            |
|                    | :green_square: | GET    | */shares/{share}/schemas/{schema}/tables/{table}/changes*          |

TODO
==============================

- [ ] API
  - [ ] CDF Related API
  - [ ] Microsoft Azure Pre-Signed URL
- [ ] Documentation
  - [x] README
  - [ ] Wiki
- [ ] DevOps
  - [x] Dockerfile
  - [ ] Helm Chart
- [ ] Admin Console (React/Frontend)
- [ ] Data Access Audit
  - [ ] Enrich Access Log
  - [ ] Share Namespaces
  - [ ] Token Blacklist

References
==============================

### Official

1. [Delta Sharing: An Open Protocol for Secure Data Sharing](https://github.com/delta-io/delta-sharing)

 You can find the Delta Sharing open protocol specification here.
 
2. [Open source self-hosted Delta Sharing server](https://delta.io/blog/2023-04-24-open-source-selfhosted-delta-sharing-server/)

 My blog post on the official Delta Lake community.

### Related Projects

1. [Riverbank](https://github.com/delta-incubator/riverbank)

 This project motivated and helped me a lot to start Kotosiro Sharing project.

2. [delta-rs](https://github.com/delta-io/delta-rs)

 Needless to say, this great Rust crate allowed me low level access to Delta tables in Rust.
