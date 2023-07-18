
## Configuration

This application requires several environment variables for configuration.

| Environment Variable | Description |
| ------ | ------ |
| `AWS_ACCESS_KEY_ID` | Your AWS Access Key. |
| `AWS_SECRET_ACCESS_KEY` | Your AWS Secret Access Key. |
| `AWS_DEFAULT_REGION` | The AWS region to connect to. |

You can set these environment variables in your shell:

```bash
export AWS_ACCESS_KEY_ID=youraccesskey
export AWS_SECRET_ACCESS_KEY=yoursecretkey
export AWS_DEFAULT_REGION=yourregion
```

Or, you could place these in a `.env` file and load them using the `dotenv` crate in Rust.

## Adjusting 'part_size' parametre

Adjusting the `part_size` parameter in the `upload_file_multipart` function allows you to customize the part size for each individual upload, providing the flexibility to adjust the setting as necessary based on the specific characteristics and requirements of each file being uploaded.

The part size is set in the `part_size` parameter of the `upload_file_multipart` function:

```rust
pub async fn upload_file_multipart(
    &self,
    bucket_name: &str,
    file_path: &str,
    file_name: &str,
    part_size: usize,
) -> Result<(), Box<dyn Error>>
```

Please be aware of the following constraints when selecting a part size:

- Each part, except the last, must be at least 5MB in size.
- The maximum part size is 5GB.
- The total number of parts in a single multipart upload is 10,000.
