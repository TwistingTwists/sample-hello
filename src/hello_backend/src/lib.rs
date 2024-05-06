use std::cell::RefCell;
use std::collections::BTreeMap;

// Define a struct for Todo items
#[derive(Debug, Clone)]
struct Todo {
    id: u32,
    title: String,
    completed: bool,
}
// ------------------------------------
// Pagination Trait
// ------------------------------------
trait Paginate {
    fn get_page(&self, page_num: usize, page_size: usize) -> Vec<(&u32, &Todo)>;
}

// Implement Pagination for BTreeMap<u32, Todo>
impl Paginate for BTreeMap<u32, Todo> {
    fn get_page(&self, page_num: usize, page_size: usize) -> Vec<(&u32, &Todo)> {
        self.iter()
            .skip((page_num - 1) * page_size)
            .take(page_size)
            .collect()
    }
}
// ------------------------------------
// Thread-local storage for todos
// ------------------------------------

thread_local! {
    static TODOS: RefCell<BTreeMap<u32, Todo>> = RefCell::new(BTreeMap::new());
}

// ------------------------------------
// CRUD Functions
// ------------------------------------
#[ic_cdk::update]
fn create_todo(title: String) {
    TODOS.with(|todos| {
        let mut map = todos.borrow_mut();
        let id = map.len() as u32 + 1;
        map.insert(
            id,
            Todo {
                id,
                title,
                completed: false,
            },
        );
        println!("Created id: {}", id);
    });
}

#[ic_cdk::query]
fn read_todos(page_num: usize, page_size: usize) {
    TODOS.with(|todos| {
        let map = todos.borrow();
        let page = map.get_page(page_num, page_size);
        if page.is_empty() {
            println!("No todos found on page {}", page_num);
        } else {
            println!("--- Page {} ---", page_num);
            for (id, todo) in page {
                println!("{}: {} (Completed: {})", id, todo.title, todo.completed);
            }
        }
    });
}

#[ic_cdk::update]
fn update_todo(id: u32, title: Option<String>, completed: Option<bool>) {
    TODOS.with(|todos| {
        let mut todos = todos.borrow_mut();
        if let Some(todo) = todos.get_mut(&id) {
            if let Some(new_title) = title {
                todo.title = new_title;
            }
            if let Some(new_completed) = completed {
                todo.completed = new_completed;
            }
        }
    })
}

#[ic_cdk::update]
fn delete_todo(id: u32) {
    TODOS.with(|todos| {
        todos.borrow_mut().remove(&id);
    })
}

// ------- TESTS -------

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
    fn test_read_todos() {
        for i in 1..=100 {
            create_todo(format!("Task {}", i));
        }

        let page_size = 10;
        let num_pages = (100 + page_size - 1) / page_size;

        // num_pages + 1 : for covering the case when page_num exceeds the bounds.
        for page_num in 1..=num_pages + 1 {
            let mut page_todos = Vec::new();
            println!("page_num: {}, num_pages: {} ", page_num, num_pages);
            TODOS.with(|todos| {
                page_todos = todos
                    .borrow()
                    .get_page(page_num, page_size)
                    .into_iter()
                    .map(|(id, todo)| (id.clone(), todo.clone()))
                    .collect::<Vec<_>>();
            });
            println!("{:<50}", "==".repeat(50));
            dbg!(page_todos
                .iter()
                .map(|(_id, todo)| todo.title.clone())
                .collect::<Vec<_>>());
            println!("{:<50}", "-".repeat(50));

            println!("page_size: {}, num_pages: {} ", page_size, num_pages);
            println!("{:<50}", "*".repeat(50));

            assert_eq!(
                page_todos.len(),
                if page_num > num_pages {
                    100 % page_size
                } else {
                    page_size
                }
            );

            for (idx, (id, todo)) in page_todos.iter().enumerate() {
                let expected_title = format!("Task {}", (page_num - 1) * page_size + idx + 1);
                assert_eq!(todo.title, expected_title);
            }
        }
    }

    #[test]
    fn test_update_todo() {
        create_todo("Test todo".to_string());
        let mut todo_id = 0;
        TODOS.with(|todos| {
            let map = todos.borrow();
            todo_id = map.keys().next().unwrap().clone();
        });

        update_todo(todo_id, Some("Updated title".to_string()), Some(true));

        let updated_todo = TODOS.with(|todos| todos.borrow().get(&todo_id).unwrap().clone());
        assert_eq!(updated_todo.title, "Updated title");
        assert_eq!(updated_todo.completed, true);
    }

    #[test]
    fn test_delete_todo() {
        create_todo("Test todo".to_string());
        let mut todo_id = 0;
        TODOS.with(|todos| {
            let map = todos.borrow();
            todo_id = map.keys().next().unwrap().clone();
        });

        delete_todo(todo_id);
        assert_eq!(TODOS.with(|todos| todos.borrow().len()), 0);
    }
}
