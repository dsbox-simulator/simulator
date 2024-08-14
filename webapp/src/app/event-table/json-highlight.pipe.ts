import { Pipe, PipeTransform } from '@angular/core';
import { DomSanitizer, SafeHtml } from '@angular/platform-browser';
import * as Prism from 'prismjs';

// Import the JSON language component
import 'prismjs/components/prism-json';

@Pipe({
  name: 'highlightJson',
  standalone: true
})
export class HighlightJsonPipe implements PipeTransform {

  constructor(private sanitizer: DomSanitizer) {}

  transform(value: any, format: boolean = false): SafeHtml {
    if (typeof value !== 'string') {
      // Format JSON with multiple lines if the format flag is true
      value = format ? JSON.stringify(value, null, 2) : JSON.stringify(value);
    }

    // Highlight JSON syntax
    const highlightedJson = Prism.highlight(value, Prism.languages['json'], 'json');
    
    // Bypass security and return the highlighted JSON as SafeHtml
    return this.sanitizer.bypassSecurityTrustHtml(highlightedJson);
  }
}
