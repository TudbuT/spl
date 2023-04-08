# "Stack Programming Language" =SPL

SPL is a simple, concise, concatenative scripting language.

Example:
```js
func main { mega | with args ;
    "Running with args: " print
    args:iter
        { str | " " concat } swap:map
        &print swap:foreach
    "" println
    println <{ "and with that, we're done" }
    0
}
```

## "5 minutes" SPL:in


- `def` introduces a variable.
  ```js
  def a
  ```
- Writing a constant pushes it to the stack. This works with strings and numbers.
  ```js
  "Hello, World!"
  ```
- Use `=<name>` to assign the topmost value to a variable. In this case, that is
  "Hello, World!"
  ```js
  =a
  ```
- This can be written as a single line - line breaks are always optional, and
  equal to a space.
  ```js
  def a "Hello, World!" =a
  ```
- Variables consist of two functions: `<name>` and `=<name>`. Use `<name>` to
  obtain the value again.
  ```js
  a
  ```
- The `print` function is used to print a value. It takes one value from the stack
  and prints it without a newline. To print with a newline, use `println`. The
  semicolon at the end means 'if this function returns anything, throw it away'.
  This can be used on strings to make them comments, but is not available for
  numeric constants.
  ```js
  println;
  ```
  ```txt
  Hello, World!
  ```
- The `func` keyword introduces a function. The `{ mega |` is the return type
  declaration, which in SPL is done within the block. In this case, our function
  returns one of the `mega` type, which is a 128-bit integer. 
  ```js
  func main { mega |
  ```
- The `with` declaration will be explained below. It defines the `args` argument.
  ```js
  with args ;
  ```
- Now, we can write code like before:
  ```js
  def list
  ```
- SPL has a varying-length array type, the list. To create any construct (object), 
  we use `:new`.
  ```js
  List:new =list
  ```
- To add to the end of a list, we `push` to it. All construct methods are
  written with a colon, like before in the `new` example.
  ```js
  "Hello," list:push
  ```
  Note the lowercase `list`, because we are pushing to the construct in the
  variable.
- Now, let's also push "World!".
  ```js
  "World" list:push
  ```
  Beautiful. I'd like to print it now, but how?
- We can't print a list directly (with what we know so far), but we can iterate
  through it!
  ```js
  { | with item ;
      item print;
      " " print;
  } list:foreach;
  "" println;
  ```
  **There is a lot to unpack here!**
  - `{ |` creates a closure with no return type (in C-style languages, that'd be
    a void function).
  - `with item ;` declares arguments. This is optional, and not needed if the
    function does not take arguments. Running `"a" "b" "c"` and calling
    something with a b c ; will leave each letter in the corresponding variable.
  - We already know what print does - it prints the item and a space in this
    case.
  - The semicolons mean we don't care about the result of printing. In this
    case, printing does not return anything, but I added the semicolons just for
    clarity or in case it did.
  - `}` ends the closure, and puts it on the top of our stack.
  - `list:foreach` calls the `foreach` method on our `list`, which is declared
    with callable this ; - that means we need to provide one argument along with
    the implied `this` argument (it can have any name - the interpreter does not
    care about names in any way - `this` is just convention). The `callable`
    here is *not* a type!
  - `foreach` also does not return anything, but I added the semicolon for
    clarity.
  - We then print a newline.
  ```txt
  Hello, World! 
  ```
- SPL has Ranges, constructed using `<lower> <upper> Range:new`. You can iterate
  over them.
  ```js
  0 5 Range:new:iter
  ```
- Now, let's multiply all of these values by 5.
  ```js
      { mega | 5 * } swap:map
  ```
  Wait, what?
  Why is there suddenly an inconsistency in method calls, the iterator isn't
  being called, it's something else now!

  It sure does look like it, doesn't it? `swap` swaps the topmost two values on
  the stack. `a b -> b a`. That means we are actually calling to our iterator.
  The closure and the iterator are swapped before the call is made. `swap:map`
  is a more concise way of writing `swap :map`.

  The map function on the iterator (which is available through `:iter` on most
  collection constructs) is used to apply a function to all items in the
  iterator. The closure here actually takes an argument, but the with
  declaration is omitted. The longer version would be:
  ```js
      { mega | with item ;
          item 5 *
      }
  ```
  But this is quite clunky, so when arguments are directly passed on to the next
  function, they are often simply kept on the stack. The `*` is simply a
  function taking two numbers and multilying them. The same goes for `+`, `-`,
  `%`, and `/`. `a b -` is equivalent to `a - b` in other languages. `lt`,
  `gt`, and `eq` are used to compare values.

  Returning is simply done by leaving something on the stack when the function
  exits, and the return declaration *can* technically be left off, but the
  semicolon won't be able to determine the amount of constructs to discard that
  way, so this should never be done unless you're absolutely sure. In this case, 
  we are absolutely sure that it will never be called with a
  semicolon, because the mapping iterator has no use for the closure other than
  the returned object (which is the case for most closures in practice.), 
  therefore we could even omit the return type declaration and get `{ | 5 *}`. 
  Neat!
- We can use `foreach` on iterators just like arrays. `_str` is used to convert
  a number to a string.
  ```js
      { | _str println } swap:foreach
  ```
  ```txt
  0
  5
  10 
  15 
  20
  ```
  Ranges are inclusive of the lower bound and exclusive in the upper bound.
  They are often used similarly to the (pseudocode) equivalent in other
  languages:
  ```java
  for(int i = 0; i < 5; i++) { println((String) i * 5); }
  ```
- SPL actually isn't fully concatenative. It supports postfix arguments as well:
  ```js
      println <{ "and with that, we're done" }
  ```
  This is actually not a special interpreter feature, more so is it a special
  lexer feature. This is 100% equivalent with the non-postfix version, where the
  string is right before the `println`.

  The same can be done for object calls. Let's rewrite the previous code with
  postfix:
  ```js
  Range:new <{ 0 5 }
      :iter
      :map <{ { | 5 * } }
      :foreach <{ { | _str println } }
  ```

  I lied. This is now no longer 100% equivalent. Let's look at what happens
  under the hood.

  ```js
  call Range
  objpush
  const mega 0
  const mega 5
  objpop
  objcall new
  objcall iter
  objpush
  const func 0
    const mega 5
    call *
    end
  objpop
  objcall map
  objpush
  const func 0
    call _str
    call println
    end
  objpop
  objcall foreach
  ```

  You can see there are now `objpush` and `objpop` instructions. This is doing
  the job that `swap` used to do in our previous example. However, swap can only
  swap the topmost values, but postfix arguments allow any amount. That's why
  there is a special instruction just for that. It can also be used through AST
  modifications, but there is no way to get it in normal language use as it can
  cause interpreter panics when they are used wrongly.

  `objpush` and `objpop` operate on a separate stack, called the objcall stack,
  as opposed to the main object stack.

More of this tutorial to follow.
