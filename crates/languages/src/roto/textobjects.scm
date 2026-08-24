(function_item
  body: (block) @function.inside) @function.around

(filtermap_item
  body: (block) @function.inside) @function.around

(test_item
  body: (block) @function.inside) @function.around

(record_item
  fields: (record_type) @class.inside) @class.around

(enum_item
  constructors: (enum_constructors) @class.inside) @class.around

(line_comment) @comment.inside
(line_comment)+ @comment.around
