//Box<T> is a smart pointer essentially, allocates memory on the heap and does automatic memory free-up following RAII concept that C++ smart pointers use
//dyn is a keyword that lets multiple implementations of a superclass run, similar to polymorphism in C++
//a trait is a way to defined shared-behvaior, again similar to superclass in C++

//Under the hood the rust compiler resolves all async functions to a future Factory

/* 
trait Future {
    type Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output>;
}
*/

// poll() method requires a mutable reference to itself that doesn't move in memory. Pin<> means the memory location is unchanged.
//Sometimes the executor may move variables around in memory, if an async method has some variable in its scope, that may get affected, hence to prevent a dangling reference the memory is pinned
/*
enum Poll<T> {
    Ready(T),     // The future is finished
    Pending,      // Not finished; try again later
}
*/
//poll() advances future by a step and it either returns Ready(val) when complete or Pending

//await gets converted as well, it keeps calling the poll() method 
/*
loop {
    match Pin::new(&mut some_future).poll(context) {
        Poll::Ready(val) => break val,
        Poll::Pending => suspend this async function and return control
    }
}

 */

 //Each await in practice pieced together forms a state-machine that is driven by the repetitive poll() command. Depending on what poll() returns, a state-transition occurs. The executor such as Tokio or async-std is a task scheduler
 //Executor's job is to hold queue of pending futures and call them synchronously and then wait via the 'await' cmd
 //Waker/context notifies the executor of when a future can continue, as in if it returns a value

use reqwest; //HTTP client lib
use serde_json::Value; //Value is any JSON type, it is dynamic & gets used so strongly-typed struct isn't required

#[tokio::main] //flag tells main function to make main function an async routine else it can't run any async functions
async fn main() -> Result<(), Box<dyn std::error::Error>> { //any type of sub-error can be returned as long as it implements method of Error trait & return pointer to this error dynamically located on heap if fails, if success then nothing
    println!("Sending request...");

    // Make an async GET request
    let response = reqwest::get("https://jsonplaceholder.typicode.com/todos/1") //await response & '?' unwraps result, if success then return it, else if error return error 
        .await? //await request
        .json::<Value>() //parse JSON
        .await?; //await parsing

    println!("Response JSON:\n{:#?}", response);

    Ok(())
}