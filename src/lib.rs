/// Example project demonstrating the use of the linked_vector crate. This was
/// originally a solution to a coding challenge on LeetCode.
/// 
/// A Least Frequently Used cache is implemented using a hash map and a linked 
/// vector of queues. The queues are also linked vectors. The cache is 
/// essentially one linked vector that holds nested linked vectors that each 
/// correspond to the number of times a key has been accessed.
/// 
/// When a new key is added to the cache, and it's already filled to capacity,
/// the least frequently used key is removed. When a key is accessed, it's 
/// frequency count is incremented, which means it's moved to the queue that
/// corresponds to the next higher frequency count.
/// 
/// What makes this problem challenging is more than one key can have the same
/// smallest frequency count, and the key that has been accessed least recently 
/// is the one that should be removed, hence the need for a queue for each 
/// frequency.
/// 
/// Both `insert()` and `get()` are O(1) operations.
/// 
/// `incr_freq()` has an example of how to use a cursor to move to specific
/// nodes in the linked vector.
/// 
/// `insert()` and `remove_lfu()` have examples of how the linked vectors can
/// be accessed through the `LinkedVector` API.
/// 

use std::collections::HashMap;
use std::hash::Hash;

use linked_vector::*;

/// A value record. It contains the value, the handle of the frequency queue
/// it belongs to and the handle of its position in that queue.
/// 
struct Value<V> {
    value : V,
    hfreq : HNode,
    hpos  : HNode,
}

impl<V> Value<V> {
    fn new(value: V) -> Self {
        Self {
            value,
            hfreq : HNode::default(), // Which frequency queue.
            hpos  : HNode::default(), // Position in the frequency queue.
        }
    }
}

/// A Least Frequently Used cache. A hash map implements the cache and queues 
/// are maintained for frequency counts.
/// 
pub struct LfuCache<K, V> {
    map         : HashMap<K, Value<V>>,
    frequencies : LinkedVector<(usize, LinkedVector<K>)>,
    capacity    : usize,
}

impl<K, V> LfuCache<K, V> 
where
    K: Eq + Hash + Clone,
{
    /// Creates a new LFU cache with the given capacity.
    /// 
    pub fn new(capacity: usize) -> Self {
        Self {
            map         : HashMap::with_capacity(capacity),
            frequencies : LinkedVector::new(),
            capacity,
        }
    }

    /// Inserts a key-value pair into the cache.
    /// 
    pub fn insert(&mut self, key: K, value: V) {
        if self.capacity == 0 { return; }
        
        if let Some(vrec) = self.map.get_mut(&key) {
            // The key already exists, update value and increment its frequency.
            vrec.value = value;
            Self::incr_freq(&mut self.frequencies, vrec);
        } else {
            // This is a new key. Remove the LFU item if the cache is full.
            if self.map.len() >= self.capacity {
                Self::remove_lfu(&mut self.frequencies, &mut self.map);
            }
            // Get the handle of the queue with frequency 1.
            let hfreq_1 = {
                if self.frequencies.front().map_or(false, |q| q.0 == 1) {
                    self.frequencies.front_node().unwrap()
                } else {
                    self.frequencies.push_front((1, LinkedVector::new()))
                }
            };
            // Create a new value record and get a mutable reference to the
            // frequency 1 queue.
            let mut vrec   = Value::new(value);
            let     freq_1 = self.frequencies.get_mut(hfreq_1);
            
            // Set the frequency queue locator handles of the value record and 
            // push its key to the frequency 1 queue.
            vrec.hfreq = hfreq_1;
            vrec.hpos  = freq_1.1.push_back(key.clone());

            // Insert the key-value pair into the map.
            self.map.insert(key, vrec);
        }
    }

    /// Returns a reference to the value corresponding to the key.
    /// 
    pub fn get(&mut self, key: &K) -> Option<&V> {
        self.map.get_mut(key).map(|vrec| {
            // Move it to the next frequency queue.
            Self::incr_freq(&mut self.frequencies, vrec);
            &vrec.value
        })
    }

    /// Removes the Least Frequently Used item from the cache.
    /// 
    fn remove_lfu(freq_qs : &mut LinkedVector<(usize, LinkedVector<K>)>, 
                  map     : &mut HashMap<K, Value<V>>) 
    {
        if let Some(hqueue) = freq_qs.front_node() {
            // Get the first queue.
            let queue = freq_qs.get_mut(hqueue);

            // Pop the first entry and remove it from the map.
            if let Some(key) = queue.1.pop_front() {
                map.remove(&key);
            }
            // If the queue is empty, remove it if it's not the first one.
            if queue.0 != 1 && queue.1.is_empty() {
                freq_qs.remove(hqueue);
            }
        }
    }

    /// Increments the frequency of the given key.
    /// 
    fn incr_freq(freq_qs : &mut LinkedVector<(usize, LinkedVector<K>)>, 
                 vrec    : &mut Value<V>) 
    {
        // Get a cursor to the frequency queue referenced by vrec.
        let mut curs   = freq_qs.cursor_mut(vrec.hfreq);
        let     hqueue = curs.node();
        let     freq   = curs.0;

        // Remove the key from it's current queue (cursor implements DerefMut).
        let key = curs.1.remove(vrec.hpos);

        if curs.move_next().is_some() && curs.0 == freq + 1 {
            // If the next queue is the one we want, add the key to it.
            vrec.hfreq = curs.node();
            vrec.hpos  = curs.1.push_back(key);
        } else {
            // If the first queue wasn't for freq + 1, create a new one.
            let mut newq = (freq + 1, LinkedVector::new());

            curs.move_to(hqueue);

            // Add the key to it and update the Value record's handles.
            vrec.hpos  = newq.1.push_back(key);
            vrec.hfreq = curs.insert_after(newq);
        }
        curs.move_to(hqueue);

        // If the former queue is empty, remove it.
        if curs.1.is_empty() {
            curs.remove();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! vec2d {
        ($( [$($x:expr),*] ),*) => (vec![$(vec![$($x),*]),*]);
    }

    #[test]
    fn test_1() {
        let null = i32::MIN;
        let cmd  = ["LfuCache","put","put","get","put","get",
                    "get","put","get","get","get"];
        let data = vec2d![[2],[1,1],[2,2],[1],[3,3],[2],[3],[4,4],[1],[3],[4]];
        let exp  = [null,null,null,1,null,-1,3,null,-1,3,4];

        let mut cache = None;

        for ((cmd, data), exp) in cmd.into_iter().zip(data).zip(exp) {
            match cmd {
                "LfuCache" => {
                    cache = Some(LfuCache::new(data[0] as usize));
                },
                "put" => {
                    if let Some(cache) = &mut cache {
                        cache.insert(data[0], data[1]);
                    } else {
                        panic!("cache is None");
                    }
                },
                "get" => {
                    if let Some(cache) = &mut cache {
                        assert_eq!(cache.get(&data[0]).map_or(-1, |v| *v), exp);
                    } else {
                        panic!("cache is None");
                    }
                },
                _ => panic!("Bad command!"),
            }
        }
    }

    #[test]
    fn test_2() {
        let null = i32::MIN;
        let cmd  = ["LfuCache","put","get"];
        let data = vec2d![[0],[0,0],[0]];
        let exp  = [null,null,-1];

        let mut cache = None;

        for ((cmd, data), exp) in cmd.into_iter().zip(data).zip(exp) {
            match cmd {
                "LfuCache" => {
                    cache = Some(LfuCache::new(data[0] as usize));
                },
                "put" => {
                    if let Some(cache) = &mut cache {
                        cache.insert(data[0], data[1]);
                    } else {
                        panic!("cache is None");
                    }
                },
                "get" => {
                    if let Some(cache) = &mut cache {
                        assert_eq!(cache.get(&data[0]).map_or(-1, |v| *v), exp);
                    } else {
                        panic!("cache is None");
                    }
                },
                _ => panic!("Bad command!"),
            }
        }
    }

    #[test]
    fn test_3() {
        let null = i32::MIN;
        let cmd  = ["LfuCache","put","get","put","get","get","get","put","get","put","put","put","put","put","put","get","put","put","get","put","put","put","put","put","put","put","get","put","put","put","put","put","put","get","put","put","get","put","get","put","get","put","put","get","put","put","put","get","get","put","put","get","put","get","get","get","get","put","get","put","put","put","get","put","put","get","get","get","put","put","put","put","get","get","get","get","put","get","put","get","put","put","get","get","get","get","get","get","put","put","get","get","put","get","put","put","get","get","put","put","put","put","put","get","put","get","get","put","get","put","put","put","put","get","put","put","put","put","get","get","get","get","put","put","get","get","put","put","put","put","put","put","put","get","put","put","get","get","put","put","put","put","put","put","put","put","put","put","put","get","put","get","get","get","put","put","put","put","put","get","put","put","get","put","put","get","put","get","put","get","put","get","get","get","get","put","get","get","get","put","put","put","get","get","get","put","put","put","put","get","get","get","put","get","get","get","get","put","get","put","put","get","get","put","put","get","get","put","put","put","get","put","get","get","put","put","put","get","put","get","put","get","put","get","put","get","put","put","get","put","put","get","put","get","get","put","get","get","get","put","get","put","put","put","put","get","get","put","get","put","get","put","get","put","get","get","get","get","get","put","put","get","put","get","put","put","get","get","put","get","get","put","get","put","put","get","put","put","get","get","put","put","get","get","put","get","put","put","get","put","put","put","get","put","put","get","get","get","get","put","put","get","get","put","put","get","put","put","put","put","get","put","get","put","get","get","put","get","put","put","get","get","put","get","put","put","get","put","get","put","put","put","put","get","get","get","put","put","put","put","get","get","put","put","get","put","put","get","put","put","get","put","put","put","put","put","get","get","put","put","put","get","put","get","get","put","put","get","put","put","put","put","put","get","put","get","get","put","get","put","put","get","get","put","put","put","put","put","put","get","put","get","get","get","get","put","get","put","put","get","put","get","put","put","get","put","put","put","put","put","put","put","put","put","put","put","get","put","get","put","get","put","put","put","get","put","get","get","put","get","get","put","get","put","put","get","get","get","get","get","put","put","put","get","put","get","put","get","get","get","put","get","put","put","put","get","get","get","get","put","put","put","put","put","put","get","get","put","put","get","put","put","get","put","get","get","get","get","put","get","get","get","put","get","get","put","put","get","put","put","put","put","get","put","put","get","put","put","put","put","put","get","put","put","put","put","put","put","put","get","put","put","put","put","put","put","get","get","put","get","get","get","get","get","put","put","put","put","put","put","put","get","put","put","get","put","get","put","put","get","put","put","get","put","put","get","get","get","get","put","put","put","get","get","put","get","put","put","put","put","get","put","put","get","get","put","put","put","put","put","put","put","put","put","get","get","put","put","get","put","put","put","put","get","put","get","get","put","get","put","get","put","put","put","put","get","put","put","get","put","put","get","put","put","get","get","put","put","get","put","put","put","put","get","get","get","put","put","get","put","get","put","put","put","put","get","put","put","put","put","get","get","put","put","get","get","put","put","put","put","get","put","put","put","get","get","get","put","put","put","put","get","put","get","get","put","put","put","put","put","get","get","put","put","put","get","put","put","put","put","get","put","put","get","get","put","put","get","get","put","put","get","put","put","put","put","get","get","get","get","put","get","get","put","put","put","get","get","put","put","get","get","get","put","get","get","get","put","get","get","get","put","put","put","put","get","put","put","put","get","get","put","get","put","put","get","get","put","get","get","get","get","put","put","put","put","put","put","put","put","get","get","put","put","put","get","put","get","get","get","put","put","get","put","put","put","put","put","put","get","put","put","put","put","get","put","put","get","put","get","put","get","put","get","put","get","put","put","put","get","put","get","get","get","get","get","put","put","put","get","put","put","put","get","get","put","put","put","put","get","put","put","put","put","put","put","get","get","put","put","put","put","put","put","get","put","put","put","get","get","put","put","put","put","put","put","put","put","put","get","put","put","get","get","get","get","put","put","get","put","get","get","get","put","put","put","put","put","put","put","get","get","put","put","put","put","put","get","get","put","put","get","put","get","put","put","put","get","put","get","put","get","get","put","get","get","get","put","put","put","put","get","put","get","put","put","put","put","put","put","get","put","put","get","put","put","get","put","put","put","put","get","put","put","get","get","put","put","put","put","get","put","get","put","put","put","put","put","put","put","put","put","get","put","put","put","put","get","get","get","put","get","get","put","put","put","put","get","put","get","get","get","put","get","get","put","get","put","get","put","put","get","put","get","get","get","get","put","put","get","put","get","get","get","get","put","put","put","get","put","get","put","get","put","get","get","put","get","get","put","put","put","put","get","get","put","put","put","get","put","get","put","put","get","put","get","put","get","put","get","get","put","get","get","put","put","put","get","put","put","get","get","put","get","put","get","put","put","get","get"];
        let data = vec2d![[105],[33,219],[39],[96,56],[129],[115],[112],[3,280],[40],[85,193],[10,10],[100,136],[12,66],[81,261],[33,58],[3],[121,308],[129,263],[105],[104,38],[65,85],[3,141],[29,30],[80,191],[52,191],[8,300],[136],[48,261],[3,193],[133,193],[60,183],[128,148],[52,176],[48],[48,119],[10,241],[124],[130,127],[61],[124,27],[94],[29,304],[102,314],[110],[23,49],[134,12],[55,90],[14],[104],[77,165],[60,160],[117],[58,30],[54],[136],[128],[131],[48,114],[136],[46,51],[129,291],[96,207],[131],[89,153],[120,154],[111],[47],[5],[114,157],[57,82],[113,106],[74,208],[56],[59],[100],[132],[127,202],[75],[102,147],[37],[53,79],[119,220],[47],[101],[89],[20],[93],[7],[48,109],[71,146],[43],[122],[3,160],[17],[80,22],[80,272],[75],[117],[76,204],[74,141],[107,93],[34,280],[31,94],[132],[71,258],[61],[60],[69,272],[46],[42,264],[87,126],[107,236],[131,218],[79],[41,71],[94,111],[19,124],[52,70],[131],[103],[81],[126],[61,279],[37,100],[95],[54],[59,136],[101,219],[15,248],[37,91],[11,174],[99,65],[105,249],[85],[108,287],[96,4],[70],[24],[52,206],[59,306],[18,296],[79,95],[50,131],[3,161],[2,229],[39,183],[90,225],[75,23],[136,280],[119],[81,272],[106],[106],[70],[73,60],[19,250],[82,291],[117,53],[16,176],[40],[7,70],[135,212],[59],[81,201],[75,305],[101],[8,250],[38],[28,220],[21],[105,266],[105],[85],[55],[6],[78,83],[126],[102],[66],[61,42],[127,35],[117,105],[128],[102],[50],[24,133],[40,178],[78,157],[71,22],[25],[82],[129],[126,12],[45],[40],[86],[100],[30,110],[49],[47,185],[123,101],[102],[5],[40,267],[48,155],[108],[45],[14,182],[20,117],[43,124],[38],[77,158],[111],[39],[69,126],[113,199],[21,216],[11],[117,207],[30],[97,84],[109],[99,218],[109],[113,1],[62],[49,89],[53,311],[126],[32,153],[14,296],[22],[14,225],[49],[75],[61,241],[7],[6],[31],[75,15],[115],[84,181],[125,111],[105,94],[48,294],[106],[61],[53,190],[16],[12,252],[28],[111,122],[122],[10,21],[59],[72],[39],[6],[126],[131,177],[105,253],[26],[43,311],[79],[91,32],[7,141],[38],[13],[79,135],[43],[94],[80,182],[53],[120,309],[3,109],[97],[9,128],[114,121],[56],[56],[124,86],[34,145],[131],[78],[86,21],[98],[115,164],[47,225],[95],[89,55],[26,134],[8,15],[11],[84,276],[81,67],[46],[39],[92],[96],[89,51],[136,240],[45],[27],[24,209],[82,145],[10],[104,225],[120,203],[121,108],[11,47],[89],[80,66],[16],[95,101],[49],[1],[77,184],[27],[74,313],[14,118],[16],[74],[88,251],[124],[58,101],[42,81],[2],[133,101],[16],[1,254],[25,167],[53,56],[73,198],[48],[30],[95],[90,102],[92,56],[2,130],[52,11],[9],[23],[53,275],[23,258],[57],[136,183],[75,265],[85],[68,274],[15,255],[85],[33,314],[101,223],[39,248],[18,261],[37,160],[112],[65],[31,240],[40,295],[99,231],[123],[34,43],[87],[80],[47,279],[89,299],[72],[26,277],[92,13],[46,92],[67,163],[85,184],[38],[35,65],[70],[81],[40,65],[80],[80,23],[76,258],[69],[133],[123,196],[119,212],[13,150],[22,52],[20,105],[61,233],[97],[128,307],[85],[80],[73],[30],[46,44],[95],[121,211],[48,307],[2],[27,166],[50],[75,41],[101,105],[2],[110,121],[32,88],[75,84],[30,165],[41,142],[128,102],[105,90],[86,68],[13,292],[83,63],[5,239],[5],[68,204],[127],[42,137],[93],[90,258],[40,275],[7,96],[108],[104,91],[63],[31],[31,89],[74],[81],[126,148],[107],[13,28],[21,139],[114],[5],[89],[133],[20],[96,135],[86,100],[83,75],[14],[26,195],[37],[1,287],[79],[15],[6],[68,11],[52],[124,80],[123,277],[99,281],[133],[90],[45],[127],[9,68],[123,6],[124,251],[130,191],[23,174],[69,295],[32],[37],[1,64],[48,116],[68],[117,173],[16,89],[84],[28,234],[129],[89],[55],[83],[99,264],[129],[84],[14],[26,274],[109],[110],[96,120],[128,207],[12],[99,233],[20,305],[26,24],[102,32],[82],[16,30],[5,244],[130],[109,36],[134,162],[13,165],[45,235],[112,80],[6],[34,98],[64,250],[18,237],[72,21],[42,105],[57,108],[28,229],[83],[1,34],[93,151],[132,94],[18,24],[57,68],[42,137],[35],[80],[10,288],[21],[115],[131],[30],[43],[97,262],[55,146],[81,112],[2,212],[5,312],[82,107],[14,151],[77],[60,42],[90,309],[90],[131,220],[86],[106,85],[85,254],[14],[66,262],[88,243],[3],[50,301],[118,91],[25],[105],[100],[89],[111,152],[65,24],[41,264],[117],[117],[80,45],[38],[11,151],[126,203],[128,59],[6,129],[91],[118,2],[50,164],[74],[80],[48,308],[109,82],[3,48],[123,10],[59,249],[128,64],[41,287],[52,278],[98,151],[12],[25],[18,254],[24,40],[119],[66,44],[61,19],[80,132],[62,111],[80],[57,188],[132],[42],[18,314],[48],[86,138],[8],[27,88],[96,178],[17,104],[112,86],[25],[129,119],[93,44],[115],[33,36],[85,190],[10],[52,182],[76,182],[109],[118],[82,301],[26,158],[71],[108,309],[58,132],[13,299],[117,183],[115],[89],[42],[11,285],[30,144],[69],[31,53],[21],[96,162],[4,227],[77,120],[128,136],[92],[119,208],[87,61],[9,40],[48,273],[95],[35],[62,267],[88,161],[59],[85],[131,53],[114,98],[90,257],[108,46],[54],[128,223],[114,168],[89,203],[100],[116],[14],[61,104],[44,161],[60,132],[21,310],[89],[109,237],[105],[32],[78,101],[14,71],[100,47],[102,33],[44,29],[85],[37],[68,175],[116,182],[42,47],[9],[64,37],[23,32],[11,124],[130,189],[65],[33,219],[79,253],[80],[16],[38,18],[35,67],[107],[88],[37,13],[71,188],[35],[58,268],[18,260],[73,23],[28,102],[129],[88],[65],[80],[119,146],[113],[62],[123,138],[18,1],[26,208],[107],[107],[76,132],[121,191],[4],[8],[117],[11,118],[43],[69],[136],[66,298],[25],[71],[100],[26,141],[53,256],[111,205],[126,106],[43],[14,39],[44,41],[23,230],[131],[53],[104,268],[30],[108,48],[72,45],[58],[46],[128,301],[71],[99],[113],[121],[130,122],[102,5],[111,51],[85,229],[86,157],[82,283],[88,52],[136,105],[40],[63],[114,244],[29,82],[83,278],[131],[56,33],[123],[11],[119],[119,1],[48,52],[47],[127,136],[78,38],[117,64],[130,134],[93,69],[70,98],[68],[4,3],[92,173],[114,65],[7,309],[31],[107,271],[110,69],[45],[35,288],[20],[38,79],[46],[6,123],[19],[84,95],[76],[71,31],[72,171],[35,123],[32],[73,85],[94],[128],[28],[38],[109],[85,197],[10,41],[71,50],[128],[3,55],[15,9],[127,215],[17],[37],[111,272],[79,169],[86,206],[40,264],[134],[16,207],[27,127],[29,48],[32,122],[15,35],[117,36],[127],[36],[72,70],[49,201],[89,215],[134,290],[77,64],[26,101],[99],[36,96],[84,129],[125,264],[43],[38],[24,76],[45,2],[32,24],[84,235],[16,240],[17,289],[49,94],[90,54],[88,199],[23],[87,19],[11,19],[24],[57],[4],[40],[133,286],[127,231],[51],[52,196],[27],[10],[93],[115,143],[62,64],[59,200],[75,85],[7,93],[117,270],[116,6],[32],[135],[2,140],[23,1],[11,69],[89,30],[27,14],[100],[61],[99,41],[88,12],[41],[52,203],[65],[62,78],[104,276],[105,307],[7],[23,123],[22],[35,299],[69],[11],[14,112],[115],[112],[108],[110,165],[83,165],[36,260],[54,73],[36],[93,69],[134],[125,96],[74,127],[110,305],[92,309],[87,45],[31,266],[10],[114,206],[49,141],[82],[92,3],[91,160],[41],[60,147],[36,239],[23,296],[134,120],[6],[5,283],[117,68],[35],[120],[44,191],[121,14],[118,113],[84,106],[23],[15,240],[37],[52,256],[119,116],[101,7],[14,157],[29,225],[4,247],[8,112],[8,189],[96,220],[104],[72,106],[23,170],[67,209],[70,39],[18],[6],[34],[121,157],[16],[19],[83,283],[13,22],[33,143],[88,133],[88],[5,49],[38],[110],[67],[23,227],[68],[3],[27,265],[31],[13,103],[116],[111,282],[43,71],[134],[70,141],[14],[119],[43],[122],[38,187],[8,9],[63],[42,140],[83],[92],[106],[28],[57,139],[36,257],[30,204],[72],[105,243],[16],[74,25],[22],[118,144],[133],[71],[99,21],[26],[35],[89,209],[106,158],[76,63],[112,216],[128],[54],[16,165],[76,206],[69,253],[23],[54,111],[80],[111,72],[95,217],[118],[4,146],[47],[108,290],[43],[70,8],[117],[121],[42,220],[48],[32],[68,213],[30,157],[62,68],[58],[125,283],[132,45],[85],[92],[23,257],[74],[18,256],[90],[10,158],[57,34],[27],[107]];
        let exp  = [null,null,-1,null,-1,-1,-1,null,-1,null,null,null,null,null,null,280,null,null,-1,null,null,null,null,null,null,null,-1,null,null,null,null,null,null,261,null,null,-1,null,-1,null,-1,null,null,-1,null,null,null,-1,38,null,null,-1,null,-1,-1,148,-1,null,-1,null,null,null,-1,null,null,-1,-1,-1,null,null,null,null,-1,-1,136,-1,null,-1,null,-1,null,null,-1,-1,153,-1,-1,-1,null,null,-1,-1,null,-1,null,null,-1,-1,null,null,null,null,null,-1,null,-1,160,null,51,null,null,null,null,-1,null,null,null,null,218,-1,261,-1,null,null,-1,-1,null,null,null,null,null,null,null,193,null,null,-1,-1,null,null,null,null,null,null,null,null,null,null,null,220,null,-1,-1,-1,null,null,null,null,null,-1,null,null,306,null,null,219,null,-1,null,-1,null,266,193,90,-1,null,-1,147,-1,null,null,null,148,147,131,null,null,null,null,-1,291,291,null,-1,178,-1,136,null,-1,null,null,147,-1,null,null,287,-1,null,null,null,-1,null,-1,183,null,null,null,174,null,110,null,-1,null,-1,null,-1,null,null,12,null,null,-1,null,89,305,null,70,-1,94,null,-1,null,null,null,null,-1,241,null,176,null,220,null,-1,null,306,-1,183,-1,12,null,null,-1,null,95,null,null,-1,-1,null,311,111,null,190,null,null,84,null,null,-1,-1,null,null,177,157,null,-1,null,null,-1,null,null,null,174,null,null,51,183,-1,4,null,null,-1,-1,null,null,21,null,null,null,null,51,null,176,null,89,-1,null,-1,null,null,176,313,null,86,null,null,229,null,176,null,null,null,null,294,110,101,null,null,null,null,128,49,null,null,82,null,null,193,null,null,193,null,null,null,null,null,-1,85,null,null,null,101,null,126,66,null,null,-1,null,null,null,null,null,-1,null,-1,67,null,66,null,null,126,101,null,null,null,null,null,null,84,null,184,23,198,110,null,101,null,null,130,null,131,null,null,130,null,null,null,null,null,null,null,null,null,null,null,239,null,35,null,-1,null,null,null,287,null,-1,240,null,313,67,null,236,null,null,121,239,299,101,105,null,null,null,118,null,160,null,135,255,-1,null,11,null,null,null,101,258,-1,35,null,null,null,null,null,null,88,160,null,null,11,null,null,276,null,291,299,90,75,null,291,276,118,null,-1,121,null,null,252,null,null,null,null,145,null,null,191,null,null,null,null,null,-1,null,null,null,null,null,null,null,75,null,null,null,null,null,null,-1,23,null,139,-1,177,165,311,null,null,null,null,null,null,null,184,null,null,309,null,100,null,null,151,null,null,109,null,null,-1,90,136,299,null,null,null,173,173,null,-1,null,null,null,null,-1,null,null,313,45,null,null,null,null,null,null,null,null,null,252,-1,null,null,212,null,null,null,null,132,null,-1,137,null,308,null,15,null,null,null,null,-1,null,null,-1,null,null,288,null,null,-1,2,null,null,22,null,null,null,null,-1,299,137,null,null,295,null,139,null,null,null,null,13,null,null,null,null,101,-1,null,null,249,190,null,null,null,null,-1,null,null,null,136,-1,151,null,null,null,null,203,null,90,88,null,null,null,null,null,190,160,null,null,null,40,null,null,null,null,24,null,null,132,30,null,null,236,-1,null,null,67,null,null,null,null,119,-1,24,132,null,1,-1,null,null,null,236,236,null,null,-1,15,183,null,311,295,183,null,-1,188,47,null,null,null,null,311,null,null,null,53,256,null,144,null,null,268,44,null,188,233,1,191,null,null,null,null,null,null,null,null,275,-1,null,null,null,53,null,138,118,146,null,null,279,null,null,null,null,null,null,175,null,null,null,null,53,null,null,-1,null,305,null,44,null,250,null,132,null,null,null,88,null,111,301,102,-1,-1,null,null,null,301,null,null,null,-1,13,null,null,null,null,-1,null,null,null,null,null,null,215,-1,null,null,null,null,null,null,233,null,null,null,311,-1,null,null,null,null,null,null,null,null,null,230,null,null,76,188,-1,264,null,null,-1,null,-1,41,-1,null,null,null,null,null,null,null,24,-1,null,null,null,null,null,47,104,null,null,287,null,24,null,null,null,93,null,-1,null,295,69,null,-1,-1,48,null,null,null,null,-1,null,-1,null,null,null,null,null,null,41,null,null,283,null,null,287,null,null,null,null,-1,null,null,299,203,null,null,null,null,296,null,13,null,null,null,null,null,null,null,null,null,276,null,null,null,null,1,-1,98,null,240,250,null,null,null,null,133,null,-1,305,-1,null,175,55,null,266,null,-1,null,null,-1,null,157,116,71,-1,null,null,-1,null,283,3,-1,102,null,null,null,106,null,240,null,-1,null,286,50,null,101,299,null,null,null,null,301,-1,null,null,null,227,null,132,null,null,144,null,279,null,71,null,68,157,null,52,24,null,null,null,268,null,null,197,3,null,25,null,54,null,null,-1,271];

        let mut cache = None;

        for ((cmd, data), exp) in cmd.into_iter().zip(data).zip(exp) {
            match cmd {
                "LfuCache" => {
                    cache = Some(LfuCache::new(data[0] as usize));
                },
                "put" => {
                    if let Some(cache) = &mut cache {
                        cache.insert(data[0], data[1]);
                    } else {
                        panic!("cache is None");
                    }
                },
                "get" => {
                    if let Some(cache) = &mut cache {
                        assert_eq!(cache.get(&data[0]).map_or(-1, |v| *v), exp);
                    } else {
                        panic!("cache is None");
                    }
                },
                _ => panic!("Bad command!"),
            }
            // To see output run tests with: 
            // cargo test -- --test-threads=1 --nocapture
            println!("cache: {:?}", cache.as_ref().unwrap().frequencies);
        }
    }

    #[test]
    fn test_4() {
        let null = i32::MIN;
        let cmd  = ["LfuCache","put","put","put","put","put","get","put","get","get","put","get","put","put","put","get","put","get","get","get","get","put","put","get","get","get","put","put","get","put","get","put","get","get","get","put","put","put","get","put","get","get","put","put","get","put","put","put","put","get","put","put","get","put","put","get","put","put","put","put","put","get","put","put","get","put","get","get","get","put","get","get","put","put","put","put","get","put","put","put","put","get","get","get","put","put","put","get","put","put","put","get","put","put","put","get","get","get","put","put","put","put","get","put","put","put","put","put","put","put"];
        let data = vec2d![[10],[10,13],[3,17],[6,11],[10,5],[9,10],[13],[2,19],[2],[3],[5,25],[8],[9,22],[5,5],[1,30],[11],[9,12],[7],[5],[8],[9],[4,30],[9,3],[9],[10],[10],[6,14],[3,1],[3],[10,11],[8],[2,14],[1],[5],[4],[11,4],[12,24],[5,18],[13],[7,23],[8],[12],[3,27],[2,12],[5],[2,9],[13,4],[8,18],[1,7],[6],[9,29],[8,21],[5],[6,30],[1,12],[10],[4,15],[7,22],[11,26],[8,17],[9,29],[5],[3,4],[11,30],[12],[4,29],[3],[9],[6],[3,4],[1],[10],[3,29],[10,28],[1,20],[11,13],[3],[3,12],[3,8],[10,9],[3,26],[8],[7],[5],[13,17],[2,27],[11,15],[12],[9,19],[2,15],[3,16],[1],[12,17],[9,1],[6,19],[4],[5],[5],[8,1],[11,7],[5,2],[9,28],[1],[2,2],[7,4],[4,22],[7,24],[9,26],[13,28],[11,26]];
        let exp  = [null,null,null,null,null,null,-1,null,19,17,null,-1,null,null,null,-1,null,-1,5,-1,12,null,null,3,5,5,null,null,1,null,-1,null,30,5,30,null,null,null,-1,null,-1,24,null,null,18,null,null,null,null,14,null,null,18,null,null,11,null,null,null,null,null,18,null,null,-1,null,4,29,30,null,12,11,null,null,null,null,29,null,null,null,null,17,-1,18,null,null,null,-1,null,null,null,20,null,null,null,29,18,18,null,null,null,null,20,null,null,null,null,null,null,null];

        let mut cache = None;

        for ((cmd, data), exp) in cmd.into_iter().zip(data).zip(exp) {
            match cmd {
                "LfuCache" => {
                    cache = Some(LfuCache::new(data[0] as usize));
                },
                "put" => {
                    if let Some(cache) = &mut cache {
                        cache.insert(data[0], data[1]);
                    } else {
                        panic!("cache is None");
                    }
                },
                "get" => {
                    if let Some(cache) = &mut cache {
                        assert_eq!(cache.get(&data[0]).map_or(-1, |v| *v), exp);
                    } else {
                        panic!("cache is None");
                    }
                },
                _ => panic!("Bad command!"),
            }
        }
    }
}
