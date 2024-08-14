export abstract class Node {
    abstract evaluate(contexts: any[]): boolean;
    
    abstract toString(): string;
}

export class LambdaNode extends Node {
    private expression: (context: any) => boolean;
    private originalExpression: string;

    constructor(expression: (context: any) => boolean, originalExpression: string) {
        super();
        this.expression = expression;
        this.originalExpression = originalExpression;
    }

    evaluate(contexts: any[]): boolean {
        for (let context of contexts) {
            try {
                console.log('Evaluating lambda node with context:', context, this.originalExpression);
                if (this.expression(context)) {
                    return true;
                }
            } catch (e) {
                console.log(e);
            }
        }
        return false;
    }

    toString(): string {
        return this.originalExpression;
    }
}


export class OperatorNode extends Node {
    private operator: string;
    private left: Node;
    private right: Node;

    constructor(operator: string, left: Node, right: Node) {
        super();
        this.operator = operator;
        this.left = left;
        this.right = right;
    }

    evaluate(contexts: any[]): boolean {
        const leftValue = this.left.evaluate(contexts);
        const rightValue = this.right.evaluate(contexts);

        if (this.operator === 'AND') {
            return leftValue && rightValue;
        } else if (this.operator === 'OR') {
            return leftValue || rightValue;
        }
        throw new Error(`Unknown operator: ${this.operator}`);
    }

    toString(): string {
        return `(${this.left.toString()} ${this.operator} ${this.right.toString()})`;
    }
}

