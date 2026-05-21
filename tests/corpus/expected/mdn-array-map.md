# Array.prototype.map()

The `map()` method of [`Array`](/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array) instances creates a new array populated with the results of calling a provided function on every element in the calling array.

## Try it

## Syntax

### Parameters

`callbackFn`

A function to execute for each element in the array. Its return value is added as a single element in the new array. The function is called with the following arguments:

`element`

The current element being processed in the array.

`index`

The index of the current element being processed in the array.

`array`

The array `map()` was called upon.

`thisArg` Optional

A value to use as `this` when executing `callbackFn`. See [iterative methods](/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array#iterative_methods).

### Return value

A new array with each element being the result of the callback function.

## Description

The `map()` method is an [iterative method](/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array#iterative_methods). It calls a provided `callbackFn` function once for each element in an array and constructs a new array from the results. Read the [iterative methods](/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array#iterative_methods) section for more information about how these methods work in general.

`callbackFn` is invoked only for array indexes which have assigned values. It is not invoked for empty slots in [sparse arrays](/en-US/docs/Web/JavaScript/Guide/Indexed_collections#sparse_arrays).

The `map()` method is [generic](/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array#generic_array_methods). It only expects the `this` value to have a `length` property and integer-keyed properties.

Since `map` builds a new array, calling it without using the returned array is an anti-pattern; use [`forEach`](/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array/forEach) or [`for...of`](/en-US/docs/Web/JavaScript/Reference/Statements/for...of) instead.

## Examples

### Mapping an array of numbers to an array of square roots

The following code takes an array of numbers and creates a new array containing the square roots of the numbers in the first array.

### Using map to reformat objects in an array

The following code takes an array of objects and creates a new array containing the newly reformatted objects.

### Using parseInt() with map()

It is common to use the callback with one argument (the element being traversed). Certain functions are also commonly used with one argument, even though they take additional optional arguments. These habits may lead to confusing behaviors. Consider:

While one might expect `[1, 2, 3]`, the actual result is `[1, NaN, NaN]`.

[`parseInt`](/en-US/docs/Web/JavaScript/Reference/Global_Objects/parseInt) is often used with one argument, but takes two. The first is an expression and the second is the radix to the callback function, `Array.prototype.map` passes 3 arguments: the element, the index, and the array. The third argument is ignored by [`parseInt`](/en-US/docs/Web/JavaScript/Reference/Global_Objects/parseInt) — but *not* the second one! This is the source of possible confusion.

Here is a concise example of the iteration steps:

To solve this, define another function that only takes one argument:

You can also use the [`Number`](/en-US/docs/Web/JavaScript/Reference/Global_Objects/Number) function, which only takes one argument:

See [A JavaScript optional argument hazard](https://wirfs-brock.com/allen/posts/166) by Allen Wirfs-Brock for more discussions.

### Mapped array contains undefined

When [`undefined`](/en-US/docs/Web/JavaScript/Reference/Global_Objects/undefined) or nothing is returned, the resulting array contains `undefined`. If you want to delete the element instead, chain a [`filter()`](/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array/filter) method, or use the [`flatMap()`](/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array/flatMap) method and return an empty array to signify deletion.

### Side-effectful mapping

The callback can have side effects.

This is not recommended, because copying methods are best used with pure functions. In this case, we can choose to iterate the array twice.

Sometimes this pattern goes to its extreme and the *only* useful thing that `map()` does is causing side effects.

As mentioned previously, this is an anti-pattern. If you don't use the return value of `map()`, use `forEach()` or a `for...of` loop instead.

Or, if you want to create a new array instead:

### Using the third argument of callbackFn

The `array` argument is useful if you want to access another element in the array, especially when you don't have an existing variable that refers to the array. The following example first uses `filter()` to extract the positive values and then uses `map()` to create a new array where each element is the average of its neighbors and itself.

The `array` argument is *not* the array that is being built — there is no way to access the array being built from the callback function.

### Using map() on sparse arrays

A sparse array remains sparse after `map()`. The indices of empty slots are still empty in the returned array, and the callback function won't be called on them.

### Calling map() on non-array objects

The `map()` method reads the `length` property of `this` and then accesses each property whose key is a nonnegative integer less than `length`.

This example shows how to iterate through a collection of objects collected by `querySelectorAll`. This is because `querySelectorAll` returns a `NodeList` (which is a collection of objects). In this case, we return all the selected `option`s' values on the screen:

You can also use [`Array.from()`](/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array/from) to transform `elems` to an array, and then access the `map()` method.

## Specifications

## Browser compatibility

## See also

- [Polyfill of `Array.prototype.map` in `core-js`](https://github.com/zloirock/core-js#ecmascript-array)
- [es-shims polyfill of `Array.prototype.map`](https://www.npmjs.com/package/array.prototype.map)
- [Indexed collections](/en-US/docs/Web/JavaScript/Guide/Indexed_collections) guide
- [`Array`](/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array)
- [`Array.prototype.forEach()`](/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array/forEach)
- [`Array.from()`](/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array/from)
- [`TypedArray.prototype.map()`](/en-US/docs/Web/JavaScript/Reference/Global_Objects/TypedArray/map)
- [`Map`](/en-US/docs/Web/JavaScript/Reference/Global_Objects/Map)
