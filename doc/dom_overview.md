# nodem-rs DOM structure

The DOM can be encoded in an XML structure, representing the DOM tree that is rendered by nodem-rs.

Each node is one styleable area on the screen.

Each node can container either, but never both:

* *Text content*, that is rendered into the node's area. The text's size determines the size of the node.
* An arbitrary number of *child nodes*, which are themselves nodes with areas and content.

The node's tagname is relevant and is not processed by nodem-rs. nodem-rs assumes <N> as the default name when outputing the DOM tree, but does not care about the tag names in the input.

# Node attributes

Generally attributes can be written without quotation marks, if their value does not contain a space.

Allowed attribute formats:

```
<N vertical>          : Just an attribute without a value.
<N padding=4>         : A single value without quotation marks.
<N padding="4 2 1 3"> : Multiple values inside quotation marks.
<N width=flex>        : Text instead of a number value.
```

Some attributes can take multiple values such as ```padding``` which can contain:

```
<N padding=2>         : Use 2 pixels at the bottom, top, left and right.
<N padding="4 2">     : Use 4 pixels on the top and bottom, 2 pixels on the left and right. 
<N padding="4 2 1 3"> : Use 4 pixels at the top, 2 pixels on the right, 1 pixel on the bottom and 3 pixels on the left.
```

Color values are numbers. 0 is usually black, 1 is white.

## Child arrangement

Child nodes will be rendered horizontally next to each other, the first one on the left, all other on the right of their previous sibling node.

This can be switched to render the nodes below each other by setting the attribute ``vertical`` on a node without a value.

## Sizing attributes

The attributes ```width``` and ```height``` can either contain the exacts pixels the node will be sized to - regardless of size - or the keyword ```flex``` which indicates that the node will take up all available free space.

The ```flex``` attribute takes a number and indicates how much of the free space in the poarent node the node gets in relation to its siblings.

## Visibility

Hide a node during rendering by setting the attribute ``visible`` to value ``0``.

## Content positioning

Nodes without a specifically set width or height will take their size from their content.

With ```margin``` an margin around the node can be set around where neither the border nor background color or other style-related elements of the node render.

With ```padding``` more space inside the node is kept around the content. This is necessary when rendering a border to prevent the content to be covered by the border.

With ```align``` the node's content is aligned in the free space around the content. The value ```start``` is the default and will render the content either left-aligned or at the top in vertical arrangement. ```end``` puts the content on the right or bottom. ```center``` leaves an equal amount of space around the content. ```stretch``` will distribute the free space equally among the node's children.

## Styling attributes

```background``` sets the background color rendered inside the node, including the padding but not the margin.

```color``` sets the text color of the node. It gets inherited to the child nodes.

```border``` selectes a border to be rendered around the node. ```0``` means no border. If a border is unavailable, a dotted 1`pixel border is rendered instead as a placeholder.

```font``` selects the font used to render text content in the node. It will also be inherited by the children.
