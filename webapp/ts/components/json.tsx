import React from "react";
import {Light as SyntaxHighlighter} from 'react-syntax-highlighter';
import json from 'react-syntax-highlighter/dist/esm/languages/hljs/json';
import theme from 'react-syntax-highlighter/dist/esm/styles/hljs/a11y-light';

SyntaxHighlighter.registerLanguage('json', json);

export function Json({json, format}: { json: any, format: boolean }) {
    return <SyntaxHighlighter language={"json"} wrapLongLines={true} style={theme}
                              customStyle={{margin: 0, padding: 0}}>
        {JSON.stringify(json, null, format ? 2 : 0)}
    </SyntaxHighlighter>;
}