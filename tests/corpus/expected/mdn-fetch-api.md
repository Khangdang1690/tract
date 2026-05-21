# Using the Fetch API

The Fetch API provides a JavaScript interface for making HTTP requests and processing the responses.

Fetch is the modern replacement for `XMLHttpRequest`: unlike `XMLHttpRequest`, which uses callbacks, Fetch is promise-based and is integrated with features of the modern web such as service workers and Cross-Origin Resource Sharing (CORS).

With the Fetch API, you make a request by calling `fetch()`, which is available as a global function in both `window` and `worker` contexts. You pass it a `Request` object or a string containing the URL to fetch, along with an optional argument to configure the request.

The `fetch()` function returns a `Promise` which is fulfilled with a `Response` object representing the server's response. You can then check the request status and extract the body of the response in various formats, including text and JSON, by calling the appropriate method on the response.

Here's a minimal function that uses `fetch()` to retrieve some JSON data from a server.

We declare a string containing the URL and then call `fetch()`, passing the URL with no extra options.

The `fetch()` function will reject the promise on some errors, but not if the server responds with an error status like `404`: so we also check the response status and throw if it is not OK.

Otherwise, we fetch the response body content as JSON by calling the `json()` method of `Response`, and log one of its values. Note that like `fetch()` itself, `json()` is asynchronous, as are all the other methods to access the response body content.

## Making a request

To make a request, call `fetch()`, passing in:

1. a definition of the resource to fetch. This can be any one of:
   - a string containing the URL
   - an object, such as an instance of `URL`, which has a stringifier that produces a string containing the URL
   - a `Request` instance
2. optionally, an object containing options to configure the request.

In this section we'll look at some of the most commonly-used options. To read about all the options that can be given, see the `fetch()` reference page.

## Setting a method

By default, `fetch()` makes a `GET` request, but you can use the `method` option to use a different request method.

## Setting a body

If the request method is `POST`, `PUT`, or `PATCH`, you can pass a body in the request.

## Setting headers

Sometimes you need to send a request with headers set, for example to set the `Content-Type` header to indicate the format of the body, or to set an `Authorization` header to provide credentials.

## Making cross-origin requests

Whether a request can be made cross-origin or not is determined by the value of the request's `mode` option.

## Including credentials

Whether the browser sends credentials, as well as the value of the `Access-Control-Allow-Credentials` response header, determines whether a cross-origin request that uses credentials will succeed or not.

## Creating a Request object

The `Request()` constructor takes the same arguments as `fetch()` itself. This means that instead of passing options to `fetch()`, you can pass the same options to the `Request()` constructor, and then pass that object to `fetch()`.

## Canceling a request

To make a request that you can cancel, create an `AbortController`, and assign its `AbortSignal` to the request's `signal` property.

## Checking request success

A `fetch()` promise will reject with a `TypeError` when a network error is encountered, although this usually means a permissions issue or similar. An accurate check for a successful `fetch()` would include checking that the promise resolved, then checking that the `Response.ok` property has a value of `true`.

## Accessing the response data

When the promise returned by `fetch()` is fulfilled, you can access the response data in various formats.

## Streaming the response

The response body is a `ReadableStream`, which lets you read the data as it arrives, in chunks.

## Uploading data

You can also use `fetch()` to upload data. The body can be a string, a `Blob`, a `FormData` object, or various other types.
