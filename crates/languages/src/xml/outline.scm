;; https://github.com/sweetppro/zed-xml/blob/main/languages/xml/outline.scm

(element
  (STag
    (Name) @name)) @item

(EmptyElemTag
  (Name) @name) @item

(doctypedecl
  (Name) @name) @item
