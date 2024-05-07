use candid::{CandidType, Decode, Deserialize, Encode};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{storable::Bound, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

// Define a struct for Todo items
#[derive(CandidType, Deserialize, Debug, Clone)]
struct Todo {
    id: u64,
    title: String,
    completed: bool,
}
// ------------------------------------
// Pagination Trait
// ------------------------------------
trait Paginate {
    fn get_page(&self, page_num: usize, page_size: usize) -> Vec<(u64, Todo)>;
}

// Implement Pagination for BTreeMap<u64, Todo>
impl Paginate for StableBTreeMap<u64, Todo, Memory> {
    // impl Paginate for BTreeMap<u64, Todo> {
    fn get_page(&self, page_num: usize, page_size: usize) -> Vec<(u64, Todo)> {
        self.iter()
            .skip((page_num - 1) * page_size)
            .take(page_size)
            .collect()
    }
}
// ------------------------------------
// storage for todos
// ------------------------------------

type Memory = VirtualMemory<DefaultMemoryImpl>;
const MAX_VALUE_SIZE: u32 = 100;

impl Storable for Todo {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: MAX_VALUE_SIZE,
        is_fixed_size: false,
    };
}
thread_local! {
     // The memory manager is used for simulating multiple memories. Given a `MemoryId` it can
    // return a memory that can be used by stable structures.
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    static TODOS: RefCell<StableBTreeMap<u64, Todo, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))),
        )
    );
    // static TODOS: RefCell<BTreeMap<u64, Todo>> = RefCell::new(BTreeMap::new());
}

// ------------------------------------
// CRUD Functions
// ------------------------------------
#[ic_cdk::update]
fn create_todo(title: String) -> u64 {
    let created_id = TODOS.with(|todos| {
        let mut map = todos.borrow_mut();
        let id = map.len() as u64 + 1;
        map.insert(
            id,
            Todo {
                id,
                title,
                completed: false,
            },
        );
        println!("Created id: {}", id);
        id
    });
    created_id
}

#[ic_cdk::query]
fn read_todos(page_num: usize, page_size: usize) -> Option<Vec<Todo>> {
    let page = TODOS.with(|todos| {
        let map = todos.borrow();
        map.get_page(page_num, page_size)
    });

    if page.is_empty() {
        println!("No todos found on page {}", page_num);
        None
    } else {
        Some(page.into_iter().map(|(_, todo)| todo).collect())
        // println!("--- Page {} ---", page_num);
        // for (id, todo) in page {
        //     println!("{}: {} (Completed: {})", id, todo.title, todo.completed);
        // }
    }
}

#[ic_cdk::update]
fn update_todo(id: u64, title: Option<String>, completed: Option<bool>) {
    TODOS.with(|todos| {
        let mut todos_mut = todos.borrow_mut();
        let mut mutable_todo = todos_mut.get(&id).unwrap();
        let mutable_todo_upd = {
            if let Some(new_title) = title {
                mutable_todo.title = new_title;
            }
            if let Some(new_completed) = completed {
                mutable_todo.completed = new_completed;
            }
            mutable_todo
        };
        todos_mut.insert(id, mutable_todo_upd);
    })
}

#[ic_cdk::update]
fn delete_todo(id: u64) {
    TODOS.with(|todos| {
        todos.borrow_mut().remove(&id);
    })
}

// ----------------------------
// ------- TESTS --------------
// ----------------------------

#[cfg(test)]
mod tests {
    use ic_cdk::println;

    use super::*;

    #[test]
    fn test_create_todo() {
        create_todo("Test todo".to_string());
        assert_eq!(TODOS.with(|todos| todos.borrow().len()), 1);
        let title = TODOS.with(|todos| {
            let map = todos.borrow();
            let todo = map.get(&1).unwrap();
            todo.title.clone()
        });
        assert_eq!(title, "Test todo");
    }

    #[test]
    fn test_read_todos_1() {
        for i in 1..=100 {
            create_todo(format!("Task {}", i));
        }
        let page_size = 10;
        let num_pages = (100 + page_size - 1) / page_size;

        // num_pages + 1 : for covering the case when page_num exceeds the bounds.
        for page_num in 1..=num_pages + 1 {
            if let Some(page_todos) = read_todos(page_num, page_size) {
                assert_eq!(
                    page_todos.len(),
                    if page_num > num_pages {
                        100 % page_size
                    } else {
                        page_size
                    }
                );

                for (idx, todo) in page_todos.iter().enumerate() {
                    let expected_title = format!("Task {}", (page_num - 1) * page_size + idx + 1);
                    assert_eq!(todo.title, expected_title);
                }
            }
        }
    }

    #[test]
    fn test_update_todo() {
        let todo_id = create_todo("Test todo".to_string());

        update_todo(todo_id, Some("Updated title".to_string()), Some(true));

        let updated_todo = TODOS.with(|todos| todos.borrow().get(&todo_id).unwrap().clone());
        dbg!(updated_todo.clone());
        assert_eq!(updated_todo.title, "Updated title");
        assert_eq!(updated_todo.completed, true);
    }

    #[test]
    fn test_delete_todo() {
        create_todo("Test todo".to_string());
        let mut todo_id = 0;
        TODOS.with(|todos| {
            let map = todos.borrow();
            todo_id = map.iter().next().map(|(k, _)| k).unwrap().clone();

            // todo_id = map.keys().next().unwrap().clone();
        });

        delete_todo(todo_id);
        assert_eq!(TODOS.with(|todos| todos.borrow().len()), 0);
    }
}

ic_cdk::export_candid!();
