export abstract class Node {
    protected lastEvaluationResult: boolean | null = null;

    abstract evaluate(contexts: any[]): boolean;
    abstract toString(): string;

    setEvaluationResult(contexts: any[]): void {
        this.lastEvaluationResult = this.evaluate(contexts);
    }

    getLastEvaluationResult(): boolean | null {
        return this.lastEvaluationResult;
    }

    // New method to collect expressions and their evaluation results
    collectExpressionsWithResults(): { expression: string, result: boolean | null }[] {
        return [{ expression: this.toString(), result: this.lastEvaluationResult }];
    }
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
                if (this.expression(context)) {
                    console.log('Evaluating lambda node with context TRUE:', context, this.originalExpression);
                    this.lastEvaluationResult = true;
                    return true;
                }
                console.log('Evaluating lambda node with context FALSE:', context, this.originalExpression);
            } catch (e) {
                console.log(e);
            }
        }
        this.lastEvaluationResult = false;
        return false;
    }

    toString(): string {
        return this.originalExpression;
    }

    // Override to return only this node's expression and result
    override collectExpressionsWithResults(): { expression: string, result: boolean | null }[] {
        console.log('Collecting expressions with results for node:', this.toString(), this.getLastEvaluationResult());
        return [{ expression: this.originalExpression, result: this.getLastEvaluationResult() }];
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

    // Override to gather expressions and results from both left and right nodes, plus the operator node itself
    override collectExpressionsWithResults(): { expression: string, result: boolean | null }[] {
        return [
            ...this.left.collectExpressionsWithResults(),
            ...this.right.collectExpressionsWithResults(),
           // { expression: this.toString(), result: this.getLastEvaluationResult() }
        ];
    }
}

export class NegationNode extends Node {
    private operand: Node;

    constructor(operand: Node) {
        super();
        this.operand = operand;
    }

    evaluate(contexts: any[]): boolean {
        const result = !this.operand.evaluate(contexts);
        this.lastEvaluationResult = result;
        return result;
    }

    toString(): string {
        return `(NOT ${this.operand.toString()})`;
    }

    override collectExpressionsWithResults(): { expression: string, result: boolean | null }[] {
        return [
            ...this.operand.collectExpressionsWithResults(),
            // { expression: this.toString(), result: this.getLastEvaluationResult() }
        ];
    }
}


export class SequenceNode extends Node {
    private left: Node;
    private right: Node;
    private leftOccurred: boolean = false;
    private fullfilled: boolean = false;

    constructor(left: Node, right: Node) {
        super();
        this.left = left;
        this.right = right;
    }

    evaluate(contexts: any[]): boolean {
        if (this.fullfilled) {
            return true;
        }
        if (!this.leftOccurred) {
            this.leftOccurred = this.left.evaluate(contexts);
            return false;
        } else {
            this.fullfilled = this.right.evaluate(contexts);
            console.log('Evaluating sequence node', this.left, this.right, this.leftOccurred, this.fullfilled);
            return this.fullfilled;
        }
    }

    toString(): string {
        return `(${this.left.toString()} -> ${this.right.toString()})`;
    }

    // Override to gather expressions and results from both left and right nodes, plus the sequence node itself
    override collectExpressionsWithResults(): { expression: string, result: boolean | null }[] {
        return [
            ...this.left.collectExpressionsWithResults(),
            ...this.right.collectExpressionsWithResults(),
           // { expression: this.toString(), result: this.getLastEvaluationResult() }
        ];
    }
}
