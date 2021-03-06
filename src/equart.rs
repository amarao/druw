use crate::fixel;
use crate::threads;
use threads::DrawingApp;
use image as im;
const WINDOW_X_START: f64 = -6.0;
const WINDOW_X_END: f64 = 6.0;
const WINDOW_Y_START: f64 = -6.0;
const WINDOW_Y_END: f64 = 6.0;

fn equart(x: f64, y:f64) -> f64{
    // x*x/y + y*y/x - 0.5
    // x.tan() - y 
    // x/1000.0 - y
    x/y/x.sin() - x*y/x.sin() - y*x.sin() + x*y.sin()
}


pub struct Equart {  // per thread instance, each instance has own 'slice' to work with
    window_start: fixel::Point,
    window_end: fixel::Point,
    fixel_size_x: f64,
    fixel_size_y: f64,
    fixels: array2d::Array2D<fixel::Fixel>,
    max_target_depth: u32,
    min_achived_depth: u32,
    // id: usize,
    // max_id: usize,
}

impl DrawingApp for Equart{
    fn new(id: usize, max_id: usize, x: u32, y: u32)-> Self {
        let slice = Self::slice(WINDOW_Y_START, WINDOW_Y_END, id, max_id);
        Self{
            window_start: fixel::Point(WINDOW_X_START, slice.0),
            window_end: fixel::Point(WINDOW_X_END, slice.1),
            fixels: array2d::Array2D::filled_with(fixel::Fixel::new(), x as usize, y as usize),
            fixel_size_x: (WINDOW_X_END - WINDOW_X_START)/x as f64,
            fixel_size_y: (slice.1 - slice.0)/y as f64,
            max_target_depth: 16,
            min_achived_depth: 4,
            // id: id,
            // max_id: max_id
        }
    }

    fn resize(&mut self, new_x: u32, new_y: u32){
        println!("redistributing fixels...");
        let mut cnt=0;
        let mut err=0;
        let mut fcnt = 0;
        let mut pcnt = 0;
        let mut new = array2d::Array2D::filled_with(fixel::Fixel::new(), new_x as usize, new_y as usize);
        let new_fixel_size_x = (self.window_end.0 - self.window_start.0)/new_x as f64;
        let new_fixel_size_y = (self.window_end.1 - self.window_start.1)/new_y as f64;
        for y in 0..self.fixels.row_len(){
            // println!("line y: {}, redistributed probes: {}, processed fixels: {}, oob err: {}", y, cnt, fcnt, err);
            for x in 0..self.fixels.column_len(){
                let fixel = self.fixels.get(x, y).unwrap();
                fcnt +=1;
                for probe in fixel{
                    pcnt += 1;
                    let new_locations = probe.gen_locations(self.window_start, self.window_end, new_fixel_size_x, new_fixel_size_y);
                    for p in new_locations{
                        if let Some(new_fixel) = new.get_mut(p[0], p[1]){
                            new_fixel.transfer_probe(probe);
                            cnt +=1;
                        }else{
                            err +=1;
                            // println!("out of bound access");
                            // println!("old fixel {}x{}, window: {:?}x{:?} fixel_size:{}x{}", x, y, self.window_start, self.window_end, new_fixel_size_x, new_fixel_size_y);
                            // println!("new fixel: [{}x{}] access to {}x{}", new_x, new_y,  p[0], p[1]);
                        }
                   }
                }
            }
        }
        println!("Done! redistributed probes: {}, processed fixels: {}, probes: {}, oob err: {}", cnt, fcnt, pcnt, err);
        for y in 0..new.row_len(){
            for x in 0..new.column_len(){
                new.get_mut(x, y).unwrap().search_roots();
            }
        }
        self.fixels = new;
        self.fixel_size_x = new_fixel_size_x;
        self.fixel_size_y = new_fixel_size_y;

        
    }

    fn get_pixel(&mut self, x: u32, y: u32) -> im::Rgba<u8> {
        const ROOT: im::Rgba<u8> = im::Rgba([0,0, 0,255]);
        const NOROOT: im::Rgba<u8> = im::Rgba([255,255,255,255]);
        const POSITIVE: im::Rgba<u8> = im::Rgba([255,255,200,255]);
        const NEGATIVE: im::Rgba<u8> = im::Rgba([200,255,255,255]);
        const OOD: im::Rgba<u8> = im::Rgba([255,0,0,255]);
        match self.fixels[(x as usize, y as usize)].root_type() {
            fixel::RootType::NoRoot => {
                match self.fixels[(x as usize, y as usize)].mood {
                    fixel::Mood::NoData => NOROOT,
                    fixel::Mood::Positive => POSITIVE,
                    fixel::Mood::Negative => NEGATIVE,
                }
            }
            fixel::RootType::Root => ROOT,
            fixel::RootType::OutOfDomain => OOD
        }
    }
    fn next_line(&mut self, _y: u32){}
    fn next_frame(&mut self){
        if self.min_achived_depth >= self.max_target_depth{
            return;
        }
        self.min_achived_depth += 1; // Issue with resizes, too much roots at once
        for y in 0..self.fixels.row_len(){
            for x in 0..self.fixels.column_len(){
                let (start, end) = self.pixel2fixel(x, y);
                self.fixels[(x, y)].add_samples(
                    equart,
                    &start, &end,
                    self.min_achived_depth
                );
            }
        }

    }
}

impl Equart{
    
    /// Convert pixel coordinates to fixel window
    fn pixel2fixel(&self, x: usize, y: usize) -> (fixel::Point, fixel::Point){
        let fixel_start_x = self.window_start.0 + self.fixel_size_x * x as f64;
        let fixel_start_y = self.window_start.1 + self.fixel_size_y * y as f64;
        let fixel_end_x = fixel_start_x + self.fixel_size_x;
        let fixel_end_y = fixel_start_y + self.fixel_size_y;
        (
            fixel::Point(fixel_start_x, fixel_start_y),
            fixel::Point(fixel_end_x, fixel_end_y)
        )
    }

    fn slice(start: f64, end: f64, id: usize, max_id: usize) -> (f64, f64){
        let span = (end - start)/max_id as f64;
        let begin =  start + span * id as f64;
        let end = begin + span;
        (begin, end)
    }

}