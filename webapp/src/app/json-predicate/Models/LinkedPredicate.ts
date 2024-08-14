import { Parser } from './Parser';
import { Tokenizer } from './Tokenizer';
import { Node } from './PredicateNode';

export class LinkedPredicate {
    public predicateNode: Node[] = [];
    public currentState: number = 0; // Index of the current predicate group
    private states: number[] = []; // List of states indicating transitions
    public endState: boolean = false;
     
  
    constructor(expression: string){      
      const conditionSections = expression.split(/\s*->\s*/);

      for (let i = 0; i < conditionSections.length; i++) {
        const tokenizer = new Tokenizer(conditionSections[i]);
        const tokens = tokenizer.tokenize();
    
        console.log('Tokens:', tokens);
        const parser = new Parser(tokens);
        const syntaxTree = parser.parse();
        this.predicateNode.push(syntaxTree);
      }

    }
  
    public evaluate(messages: any[]): boolean {
      console.log('Evaluating linked predicate', this.currentState, this.predicateNode.length);
      if (this.currentState >= this.predicateNode.length) {
        return this.endState; // No more predicate to evaluate
      }
  
      const currentGroup = this.predicateNode[this.currentState];
      const result = currentGroup.evaluate(messages);
  
      if (result) {
        // Move to the next state
        this.currentState++;
        if (this.currentState >= this.predicateNode.length) {
          //We are in the end state
          this.endState = true;
          return true;
        }
        return false;
      }
  
      return false;
    }

    public reset() {
      this.currentState = 0;
      this.endState = false;
    }
  }
  