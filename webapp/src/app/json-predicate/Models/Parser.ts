import { LambdaNode, OperatorNode, SequenceNode, Node, NegationNode } from "./PredicateNode";

export class Parser {
    private tokens: string[];
    private currentPosition: number;

    constructor(tokens: string[]) {
        this.tokens = tokens;
        this.currentPosition = 0;
    }

    parse(): Node {
        const output: Node[] = [];
        const operators: string[] = [];

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

    private addMsgPrefix(expression: string): string {
        const regex = /(?<!\w)([a-zA-Z_]\w*(?:\.[a-zA-Z_]\w*)*)(?=\s*(?:\(|\.|\s*(?:===|!==|<=|>=|<|>|=|!=|AND|OR|->)))/g;
        return expression.replace(regex, (match) => {
            if (match === 'true' || match === 'false' || /^['"].*['"]$/.test(match)) {
                return match;
            }
            return `zdjnfprefix.${match}`;
        });
    }
}
