In ToyLang there are two types of functions, lambdas and regular functions.
Take these two
```toy
let b = 3;
fn add(a: int): int{
    return a + b; //This is invalid, normal functions cannot capture like that
}
let add = (a: int): int {
    return a + b; //valid
}
```
These lambdas are first class expressions and so in the same way there is an idx node or a ref node in the ast there should be a call node that takes a generic expression of the lambda type
