import { Parser } from './Parser';
import { Tokenizer } from './Tokenizer';
import { Node } from './PredicateNode';

/**
 * Creates and Stores the syntaxTree for the given expression
 */
export class LinkedPredicate {
    public endState: boolean = false;
    public syntaxTree: Node;
     
  
    constructor(expression: string){     
      const conditionSections = expression.split(/\s*->\s*/);

      const tokenizer = new Tokenizer(expression);
      const tokens = tokenizer.tokenize();
  
      //console.log('Tokens:', tokens);
      const parser = new Parser(tokens);
      this.syntaxTree = parser.parse();      

    }
  
    public evaluate(messages: any[]): boolean {
      //console.log('Evaluating linked predicate', this.currentState, this.predicateNode.length);
      this.syntaxTree.setEvaluationResult(messages);
      return this.syntaxTree.getLastEvaluationResult() || false;
    }

    public reset() {
      this.endState = false;
    }

    public toString(): string {
        return `(${this.syntaxTree.toString()})`;
    }
  }
  