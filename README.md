# Checkpoint Log

06/23/2025: Decided to only stock Message and File stores and forgo Database stores as a result of limited time. 
The current step is to create a Data Supplier Module to fulfill supplies for stock as they are being parsed in the Recipe module. So, whenever a user asks for a `Name()` then the Data Supplier will provide a random name.

## Goals
- A total of 21 rules to understand the DSL.

### Glossary
Inventory: A collection of stock orders. This is passed to a Stockboy.
Stock orders: A store to supplied with specified stock items
Stock Item: A record to be stored on a shelf with key value pairs.
Shelf: A particular data location in a store.
Record: An entity in a store which may have attributes or values.
Label: A key in a stock item or created by a user with an `@` sign forllowed by alphanumeric letters.
Modifiers: Change how stockboy load the store or shelf.

### Rules
1. Any table populated will first be truncated
2. A table dependencies will be populated before the table itself.

3. Each column key creates a label.
4. A column key can be given a user specified label and the column key will not become a label.
5. A label can be created by a user.
6. All labels share a global stock order namespace and must be unique.
7. A user can assign a value to a label with the stockboy API or by assigning a value to column key. 
8. If a label is left unassigned then stockboy will assign a value.
9. Each stock item creates a unique record in a store. 
10. The value of a stock item's key can be a literal or label.
11. A label created in an itemized stock order will have a variable value on each iteration and is inaccessible from outside the itemzied context.

#### Obselete Rules
- Any unique column tuple will correspond to a unique row in a table.
    - On the first usage of a unique column tuple, a corresponding row
    will be created
    - Any subsequent usage of any value in a unique column tuple, the
    referent will reference the unique row of the unique column tuple
