
// use url::{Url};
use min_max_heap::{ MinMaxHeap};
use std::time::{SystemTime};
use std::path::Path;
use tokio::fs::{remove_file};

enum CacheResult {
    Saved,
    Not_Saved,
}

pub struct CacheNode {
    uid: String,
    timestamp: SystemTime,
    size: u64,
}

fn url_to_path(uid: String) -> Path {
    let (path,file) = uid.split_once('#').unwrap();
    let ppath = Path::new(path);
    
    let fpath = ppath.join(file);
    return fpath.as_path();

}

pub struct LocalCacheFacade<'a> {
    destination: &'a Path,
    limit:u64,
    heap: MinMaxHeap<CacheNode>

}

impl<'a> LocalCacheFacade<'a> {
    pub fn new(destination: &Path, limit: u64) -> Self {
        let this = Self{
            destination:destination,
            limit:limit,
            heap:MinMaxHeap::<CacheNode>::new()
        };

        this
    }

    pub fn write(&mut self, uid: String, src: &[u8]) -> Result<CacheResult,&'static str>{

        // early exit if data is bigger than limit
        if src.len() as u64 > self.limit {
            return  Ok(CacheResult::Not_Saved) ;
        }

        let used_size = self.heap.iter().map(|x| x.size).fold(0, |acc, x| acc+x);
        
        if
        // if there's still room
        if used_size + src.len() as u64 > self.limit {
            let delta = (used_size+src.len())-self.limit;

            for caheitem in self.heap.drain_asc() {
                remove_file(path)
            }
            while delta>0 {

            }
            .
            // from min of mean delete files until deletwed size >= (used_size+src.len())-self.limit 
        }

        // write file
        // add to heap
        self.heap.
    }

    pub fn read(&mut self, uid: String) -> Option<&[u8]> {

    }
}