
// use url::{Url};
use min_max_heap::{ MinMaxHeap, DrainAsc};
use tokio::io::AsyncWriteExt;
use std::cmp::Ordering;
use std::error::Error;
use std::time::{SystemTime};
use std::path::{Path, PathBuf};
use tokio::fs::{remove_file, OpenOptions};

// cannot access the necessary private member of MinMaxHeap
// to implement the update of a existing Node

// pub trait Updatable {
//     fn update(&mut self) -> Option<bool>;
// }

// impl<T> Updatable for MinMaxHeap<T> {
//     fn update(&mut self) -> Option<bool> {
//         // find and update the node in the vector
//         self.
//         // Bubble up the node
//         Hole::new(&mut self.0, pos).bubble_up();

//     }
// }

pub enum CacheResult {
    Saved,
    NotSaved,
}

// #[derive(PartialEq, Eq, PartialOrd, Ord)]
#[derive(Clone)]
pub struct CacheNode {
    uid: String,
    timestamp: SystemTime,
    size: u64,
}

impl Ord for CacheNode {
    fn cmp(&self, other: &Self) -> Ordering {
        self.timestamp.cmp(&other.timestamp)
    }
}

impl PartialOrd for CacheNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.timestamp.partial_cmp(&other.timestamp)
    }
}

impl PartialEq for CacheNode {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp.eq(&other.timestamp)
    }
}

impl Eq for CacheNode {}

// fn url_to_path(uid: String) -> &'static Path {
fn url_to_path(uid: String) ->  PathBuf {
    // /crawler5-2023-05-11.tar.gz#111-225
    // /crawler5-2023-05-11.tar.gz/111-225
    let (path,file) = uid.split_once('#').unwrap();
    let ppath = Path::new(path);
    ppath.join(file)
    // let fpath = ppath.join(file);
    // fpath;

}

pub struct LocalCacheFacade<'a> {
    destination: &'a Path,
    limit:u64,
    heap: MinMaxHeap<CacheNode>

}


impl<'a> LocalCacheFacade<'a> {
    pub fn new(destination: &'a Path, limit: u64) -> Self {
        let this = Self{
            destination:destination,
            limit:limit,
            heap:MinMaxHeap::<CacheNode>::new()
        };

        this
    }

    pub async fn write(&mut self, uid: String, src: &[u8]) -> Result<CacheResult,&'static str>{

        // early exit if data is bigger than limit
        if src.len() as u64 > self.limit {
            return  Ok(CacheResult::NotSaved) ;
        }

        let used_size = self.heap.iter().map(|x| x.size).fold(0, |acc, x| acc+x);
        
        // if there's no more room
        if used_size + src.len() as u64 > self.limit {
            let mut delta = (used_size+src.len() as u64)-self.limit;


            self.heap.drain_asc().take_while(|x|{ delta -= x.size; delta>0 }).for_each(|x| { remove_file(x.uid); });

            
        }

        let mut file = OpenOptions::new()
                .create(true)
                .open(url_to_path(uid.clone()))
                .await.expect("Open file fails"); 
        
        match file.write_all(src).await {
            Ok(_) =>{
                self.heap.push(CacheNode { uid: uid, timestamp: SystemTime::now(), size: src.len() as u64 });
                Ok(CacheResult::Saved)
            },
            Err(_) => Ok(CacheResult::NotSaved),
        }
        
    }

    pub fn read(&mut self, uid: String) -> Option<&[u8]> {

        // self.heap.

        return None;

    }
}

mod local_cache_test {
    use super::*;
    
    #[test]
    fn cache_heap_ordering() {
        // the CacheNode should be compared by their timestamp,
        // So since '3' was the last inserted it has the biggest Timestamp of all
        // So it should be the max element followed in revers order on push by all other elements
        let mut heap = MinMaxHeap::<CacheNode>::new();
        heap.push(CacheNode { uid: "1".to_string(), timestamp: SystemTime::now(), size: 5 });
        heap.push(CacheNode { uid: "2".to_string(), timestamp: SystemTime::now(), size: 5 });
        heap.push(CacheNode { uid: "3".to_string(), timestamp: SystemTime::now(), size: 5 });

        let mut i = heap.len();

        for elem in heap.drain_desc() {
            assert_eq!(i.to_string(),elem.uid );
            i-=1;
        }
    }

    #[test]
    fn cache_heap_update() {
        let mut heap = MinMaxHeap::<CacheNode>::new();
        heap.push(CacheNode { uid: "1".to_string(), timestamp: SystemTime::now(), size: 5 });
        heap.push(CacheNode { uid: "3".to_string(), timestamp: SystemTime::now(), size: 5 });
        heap.push(CacheNode { uid: "2".to_string(), timestamp: SystemTime::now(), size: 5 });


        // Update elem with uid:"3" to current timestamp
        let mut tmp = heap.into_vec();
        tmp.iter_mut().find(|x| x.uid == "3").and_then(|x| {x.timestamp = SystemTime::now(); Some(x)});
        heap = MinMaxHeap::<CacheNode>::
        heap.iter().find(|x| x.uid == "3").and_then(|x| {x.timestamp = SystemTime::now(); Some(x)});

        let mut i = heap.len();

        for elem in heap.drain_desc() {
            assert_eq!(i.to_string(),elem.uid );
            i-=1;
        }
    }
}
