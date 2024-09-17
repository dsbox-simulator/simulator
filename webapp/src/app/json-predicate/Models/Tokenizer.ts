/**
 * Tokenizer class for splitting an expression into tokens.
 */
export class Tokenizer {
    private expression: string;
    private placeholderOpen: string = '__ESCAPED_OPEN__';
    private placeholderClose: string = '__ESCAPED_CLOSE__';

    constructor(expression: string) {
        this.expression = expression;
    }

    tokenize(): string[] {
        // Temporarily replace escaped parentheses with placeholders
        let modifiedExpression = this.expression.replaceAll('\\(', this.placeholderOpen);
        modifiedExpression = modifiedExpression.replaceAll('\\)', this.placeholderClose);

        // Replace normal parentheses with space-padded versions
        modifiedExpression = modifiedExpression.replaceAll(/([()])/g, ' $1 ');

        // Restore the placeholders to the original escaped parentheses
        modifiedExpression = modifiedExpression.replaceAll(this.placeholderOpen, '(');
        modifiedExpression = modifiedExpression.replaceAll(this.placeholderClose, ')');

        // Split by spaces and filter out empty tokens
        return modifiedExpression.split(/\s+/).filter(token => token.length > 0);
    }
}
