# Request-Response

## Classes Involved In ActiveRequest to ActiveResponse Stream Communication

User has send request to the server and receives a stream of responses.

```mermaid
classDiagram
    Client "1" --> "1" DataSegment: stores request payload
    Server "1" --> "1" DataSegment: stores response payload
    Client "1" --> "1..*" ActiveRequest
    Server "1" --> "1..*" ActiveResponse
    ActiveRequest "1" --> "1" ZeroCopyConnection: receive response
    ActiveResponse "1" --> "1" ZeroCopyConnection: send response
```

## Sending Request: Client View

```mermaid
sequenceDiagram
    User->>+Client: loan
    Client-->>-User: RequestMut
    create participant RequestMut
    User->>RequestMut: write_payload
    destroy RequestMut
    User->>+RequestMut: send
    RequestMut-->>-User: ActiveRequest
    create participant ActiveRequest
    User->>+ActiveRequest: receive
    ActiveRequest-->>-User: Response
    create participant Response
    User->>Response: read_payload
    destroy ActiveRequest
    User->>ActiveRequest: drop
```

## Responding: Server View

```mermaid
sequenceDiagram
    User->>+Server: receive
    Server-->>-User: ActiveResponse
    create participant ActiveResponse
    User->ActiveResponse: read_payload
    User->>+ActiveResponse: loan
    ActiveResponse-->>-User: ResponseMut
    create participant ResponseMut
    User->>ResponseMut: write_payload
    destroy ResponseMut
    User->>ResponseMut: send
    destroy ActiveResponse
    User->>ActiveResponse: drop
```
