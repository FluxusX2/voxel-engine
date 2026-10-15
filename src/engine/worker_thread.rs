use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use crate::world::chunk::Chunk;
use crate::world::terrain;

type ChunkCoords = (i32, i32);

pub struct ChunkWorker {
    requests: Sender<(ChunkCoords, u32)>,
    results: Receiver<(ChunkCoords, Chunk)>,
}

impl ChunkWorker {
    pub(crate) fn new() -> Self {
        let (request_tx, request_rx) = mpsc::channel::<(ChunkCoords, u32)>();
        let (result_tx, result_rx) = mpsc::channel::<(ChunkCoords, Chunk)>();

        thread::spawn(move || {
            while let Ok(((x, z), seed)) = request_rx.recv() {
                let chunk = terrain::generate_chunk(x, z, seed);
                if result_tx.send(((x, z), chunk)).is_err() {
                    break; // The app has shut down.
                }
            }
        });

        Self {
            requests: request_tx,
            results: result_rx,
        }
    }

    pub(crate) fn request(&self, coords: ChunkCoords, seed: u32) {
        let _ = self.requests.send((coords, seed));
    }

    pub(crate) fn try_result(&self) -> Option<(ChunkCoords, Chunk)> {
        self.results.try_recv().ok()
    }
}