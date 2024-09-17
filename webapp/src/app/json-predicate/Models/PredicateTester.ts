import { Parser } from "./Parser";
import { Tokenizer } from "./Tokenizer";

/**
 * PredicateTester class that tests the predicate parser. Not used in the application.
 */
export class PredicateTester {
    private expression: string;
    private messages: any[][];

    constructor() {
        this.expression = '(test === 1 AND test2 === 2) OR foo === "bar" -> foo === "ja" AND test >= 2 -> test === 1 AND (test2 === 2 OR foo === "bar")';
        
        // Array von Arrays von JSON-Nachrichten
        this.messages = [
            [
                { test: 1, test2: 2, foo: "bar" },
                { test: 1, test2: 3, foo: "bar" },
                { test: 1, test2: 2, foo: "baz" },
                { test: 2, test2: 2, foo: "bar" }
            ],
            [
                { test: 1, test2: 2, foo: "bar" },
                { test: 3, test2: 4, foo: "baz" }
            ]
        ];
    }

    public runTests() {
    }

    public runTests2(){
        const expressions = [
            "(test === 1 AND (test2 === '1' OR test2 === '2')) OR foo.startsWith\\('b'\\) AND test === 12",
            "((test !== 5 AND test < 10) OR (test2 === '1' AND foo === 'baz'))",
            "(foo === 'bar' OR foo === 'baz') AND (test > 5 AND test2 !== '2')",
            "(test <= 10 AND test >= 1) OR (foo === 'qux' AND test2 === '4')",
            "(foo === 'bar' OR foo === 'baz') AND (test === 12 OR test === 1)",
            "test === 12 AND (foo === 'bar' OR foo === 'baz') AND (test2 === '1' OR test2 === '2')",
            "(foo.startsWith\\('b'\\) AND test === 1) OR (test2.includes\\('2'\\) AND foo === 'qux')",
            "foo.endsWith\\('z'\\) AND (test > 10 OR test2 === '3')",
            "(foo.includes\\('a'\\) AND test2 === '5') OR test !== 3",
            "(foo.startsWith\\('qu'\\) AND test2.endsWith\\('x'\\)) OR (test2.includes\\('1'\\) AND foo !== 'baz')",
            "test === 1 AND test === 12",
            "test2 === '1' && foo === 'bar' AND test === 1",
            "test.something === 1",
        ];
        
        const data = [
            { test: 1, test2: '1', foo: 'bar' },
            { test: 12, test2: '3', foo: 'baz' },
            { test: 12, foo: 'bar', test2: '1' },
            { test: 5, test2: '2', foo: 'qux' },
            { test: 6, test2: '4', foo: 'bar' },
            { test: 8, test2: '2', foo: 'baz' },
            { test: 2, test2: '1', foo: 'bar' },
            { test: 7, test2: '3', foo: 'baz' },
            { test: 3, test2: '5', foo: 'qux' },
            { test: 10, test2: '4', foo: 'baz' },
            { test: 1, test2: '2', foo: 'bax' },
            { test: 15, test2: '2', foo: 'qux' },
            { test: 3, test2: '5', foo: 'baz' },
            { test: 3, test2: '1', foo: 'baz' },
            { test: {
                something: 1,
            } },

        ];
        
        expressions.forEach(expression => {
            const tokenizer = new Tokenizer(expression);
            const tokens = tokenizer.tokenize();
        
            const parser = new Parser(tokens);
            const syntaxTree = parser.parse();
        
            const filteredData = syntaxTree.evaluate(data);
           // const filteredData = data.filter(msg => syntaxTree.evaluate(msg));
        
            console.log(`Expression: ${expression}`);
            console.log('Filtered Data:', filteredData);
            console.log('Syntax Tree:', syntaxTree);
            console.log('======================');
        });
               
    }
}
