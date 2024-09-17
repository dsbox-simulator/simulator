import { LambdaNode, OperatorNode, SequenceNode, Node, NegationNode } from "./PredicateNode";

/**
 * Parser class that parses tokens into a syntax tree.
 */
export class Parser {
    private tokens: string[];
    private currentPosition: number;

    /**
     * 
     * @param tokens String array of tokens to parse.
     */
    constructor(tokens: string[]) {
        this.tokens = tokens;
        this.currentPosition = 0;
    }

    /**
     * 
     * @returns The syntax tree.
     */
    parse(): Node {
        const output: Node[] = [];
        const operators: string[] = [];
        // Priority of operators
        const precedence: Record<string, number> = { 'NOT': 3, 'AND': 2, 'OR': 1, '->': 0 }; 

        const applyOperator = () => {
            const operator = operators.pop();
            if (!operator) return;
            if (operator === 'NOT') {
                const right = output.pop();
                if (!right) return;
                output.push(new NegationNode(right));
            } else {
                const right = output.pop();
                const left = output.pop();
                if (!left || !right) return;
                if (operator === '->') {
                    output.push(new SequenceNode(left, right));
                } else {
                    output.push(new OperatorNode(operator, left, right));
                }
            }
        };

        while (this.currentPosition < this.tokens.length) {
            const token = this.tokens[this.currentPosition];

            if (token === '(') {
                operators.push(token);
            } else if (token === ')') {
                while (operators.length && operators[operators.length - 1] !== '(') {
                    applyOperator();
                }
                operators.pop();
            } else if (['NOT', 'AND', 'OR', '->'].includes(token)) {
                while (operators.length && precedence[operators[operators.length - 1]] >= precedence[token]) {
                    applyOperator();
                }
                operators.push(token);
            } else {
                const expressionTokens = [token];
                while (
                    this.currentPosition + 1 < this.tokens.length &&
                    !['NOT', 'AND', 'OR', '->', '(', ')'].includes(this.tokens[this.currentPosition + 1])
                ) {
                    this.currentPosition++;
                    expressionTokens.push(this.tokens[this.currentPosition]);
                }

                const expression = expressionTokens.join(' ');
                const modifiedExpression = this.addMsgPrefix(expression);
                const lambda = (new Function('zdjnfprefix', `return ${modifiedExpression};`)) as (context: any) => boolean;
                output.push(new LambdaNode(lambda, expression));
            }

            this.currentPosition++;
        }

        while (operators.length) {
            applyOperator();
        }

        return output[0];
    }

    /**
     * Adds a prefix to message fields in the expression.
     * 
     * @param expression The expression to add the prefix to.
     * @returns The expression with the prefix added.
     */
    private addMsgPrefix(expression: string): string {
        const regex = /(?<!\w)([a-zA-Z_]\w*(?:\.[a-zA-Z_]\w*)*)(?=\s*(?:\(|\.|\s*(?:===|!==|<=|>=|<|>|=|!=|AND|OR|->)))/g;
        return expression.replace(regex, (match) => {
            if (match === 'true' || match === 'false' || /^['"].*['"]$/.test(match)) {
                return match;
            }
            return `zdjnfprefix.data.msg.${match}`;
        });
    }
}
