use super::super::facades;
use deadpool_postgres::{Object};
use facades::compression::gzip_compress;
use facades::postgres_facade::get_offset;

/*Steps
1. Compress bytes
2. Ask BD for current offset
3. Create file name
4. Send to EFS
*/
pub async fn save(bytes: Vec<u8>, client: Object) -> Result<String, String> {
    
    //Compress
    match gzip_compress(bytes) {
        Ok(compressed) => {
            let offsets = get_offset(client, compressed.len()).await;

            offsets
                .map(|o| format!("nom_fichier_{start}_{stop}.gz", start = o.0, stop = o.1))
                .and_then(|file_name| ())
                .map_err(|err| err.to_string())

            todo!()
        }
        Err(_) => todo!(),
    }

    //Ask bd
    let offsets = get_offset(client, )

    todo!()
}

pub fn get(reference: String) {

}