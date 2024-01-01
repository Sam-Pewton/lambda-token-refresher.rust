# lambda-token-refresher.rust
AWS Lambda function token refresher written in Rust for a personal project.

I wrote this Lambda function as both an experiment to use Rust in an AWS Lambda function, and to
also use as part of a personal project.

I kept most of the function names and inputs fairly generic here, but if you would like to use this
function yourself you will need to make some modifications to fit your setup, such as the payload 
that is sent from AWS EventBridge that is ingested by the function. This is where most of the 
parameters used by the function come from, such as parameter names and tokens.

### Python
In order to build this project, the [`cargo-lambda`](https://www.cargo-lambda.info/) package is 
required. You can use the `requirements.txt` file in this project to install this.
```bash
pip install -r ./requirements.txt
```

## Building the application
To build the program and contain in a .zip file:
```bash
cargo lambda build --release && zip -j ../rust.zip ./target/lambda/lambda-token-refresher/bootstrap
```
Could also be read from AWS S3 if required.

Can be uploaded manually or via Terraform.
