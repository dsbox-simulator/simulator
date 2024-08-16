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
                if (this.expression(context)) {

                console.log('Evaluating lambda node with context TRUE:', context, this.originalExpression);
                    return true;
                }

                console.log('Evaluating lambda node with context FALSE:', context, this.originalExpression);
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

export class SequenceNode extends Node {
    private left: Node;
    private right: Node;
    private leftOccurred: boolean;
    private fullfilled: boolean;

    constructor(left: Node, right: Node) {
        super();
        this.left = left;
        this.right = right;
        this.leftOccurred = false;
        this.fullfilled = false;
    }

    evaluate(contexts: any[]): boolean {

        //console.log('Evaluating sequence node', this.left, this.right, this.leftOccurred, this.fullfilled);
        if(this.fullfilled){
            return true;
        }
        if(!this.leftOccurred){
            this.leftOccurred = this.left.evaluate(contexts);
            return false;
        }else{
            this.fullfilled =  this.right.evaluate(contexts);
            console.log('Evaluating sequence node', this.left, this.right, this.leftOccurred, this.fullfilled);
            return this.fullfilled;
        }
    }

    toString(): string {
        return `(${this.left.toString()} -> ${this.right.toString()})`;
    }
}
