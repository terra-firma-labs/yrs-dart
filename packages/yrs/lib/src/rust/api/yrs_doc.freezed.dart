// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'yrs_doc.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;

/// @nodoc
mixin _$YrsPathSegment {
  Object get field0;

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is YrsPathSegment &&
            const DeepCollectionEquality().equals(other.field0, field0));
  }

  @override
  int get hashCode =>
      Object.hash(runtimeType, const DeepCollectionEquality().hash(field0));

  @override
  String toString() {
    return 'YrsPathSegment(field0: $field0)';
  }
}

/// @nodoc
class $YrsPathSegmentCopyWith<$Res> {
  $YrsPathSegmentCopyWith(YrsPathSegment _, $Res Function(YrsPathSegment) __);
}

/// Adds pattern-matching-related methods to [YrsPathSegment].
extension YrsPathSegmentPatterns on YrsPathSegment {
  /// A variant of `map` that fallback to returning `orElse`.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case _:
  ///     return orElse();
  /// }
  /// ```

  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(YrsPathSegment_Key value)? key,
    TResult Function(YrsPathSegment_Index value)? index,
    required TResult orElse(),
  }) {
    final _that = this;
    switch (_that) {
      case YrsPathSegment_Key() when key != null:
        return key(_that);
      case YrsPathSegment_Index() when index != null:
        return index(_that);
      case _:
        return orElse();
    }
  }

  /// A `switch`-like method, using callbacks.
  ///
  /// Callbacks receives the raw object, upcasted.
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case final Subclass2 value:
  ///     return ...;
  /// }
  /// ```

  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(YrsPathSegment_Key value) key,
    required TResult Function(YrsPathSegment_Index value) index,
  }) {
    final _that = this;
    switch (_that) {
      case YrsPathSegment_Key():
        return key(_that);
      case YrsPathSegment_Index():
        return index(_that);
    }
  }

  /// A variant of `map` that fallback to returning `null`.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case _:
  ///     return null;
  /// }
  /// ```

  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(YrsPathSegment_Key value)? key,
    TResult? Function(YrsPathSegment_Index value)? index,
  }) {
    final _that = this;
    switch (_that) {
      case YrsPathSegment_Key() when key != null:
        return key(_that);
      case YrsPathSegment_Index() when index != null:
        return index(_that);
      case _:
        return null;
    }
  }

  /// A variant of `when` that fallback to an `orElse` callback.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case _:
  ///     return orElse();
  /// }
  /// ```

  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? key,
    TResult Function(int field0)? index,
    required TResult orElse(),
  }) {
    final _that = this;
    switch (_that) {
      case YrsPathSegment_Key() when key != null:
        return key(_that.field0);
      case YrsPathSegment_Index() when index != null:
        return index(_that.field0);
      case _:
        return orElse();
    }
  }

  /// A `switch`-like method, using callbacks.
  ///
  /// As opposed to `map`, this offers destructuring.
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case Subclass2(:final field2):
  ///     return ...;
  /// }
  /// ```

  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) key,
    required TResult Function(int field0) index,
  }) {
    final _that = this;
    switch (_that) {
      case YrsPathSegment_Key():
        return key(_that.field0);
      case YrsPathSegment_Index():
        return index(_that.field0);
    }
  }

  /// A variant of `when` that fallback to returning `null`
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case _:
  ///     return null;
  /// }
  /// ```

  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? key,
    TResult? Function(int field0)? index,
  }) {
    final _that = this;
    switch (_that) {
      case YrsPathSegment_Key() when key != null:
        return key(_that.field0);
      case YrsPathSegment_Index() when index != null:
        return index(_that.field0);
      case _:
        return null;
    }
  }
}

/// @nodoc

class YrsPathSegment_Key extends YrsPathSegment {
  const YrsPathSegment_Key(this.field0) : super._();

  @override
  final String field0;

  /// Create a copy of YrsPathSegment
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $YrsPathSegment_KeyCopyWith<YrsPathSegment_Key> get copyWith =>
      _$YrsPathSegment_KeyCopyWithImpl<YrsPathSegment_Key>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is YrsPathSegment_Key &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'YrsPathSegment.key(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $YrsPathSegment_KeyCopyWith<$Res>
    implements $YrsPathSegmentCopyWith<$Res> {
  factory $YrsPathSegment_KeyCopyWith(
          YrsPathSegment_Key value, $Res Function(YrsPathSegment_Key) _then) =
      _$YrsPathSegment_KeyCopyWithImpl;
  @useResult
  $Res call({String field0});
}

/// @nodoc
class _$YrsPathSegment_KeyCopyWithImpl<$Res>
    implements $YrsPathSegment_KeyCopyWith<$Res> {
  _$YrsPathSegment_KeyCopyWithImpl(this._self, this._then);

  final YrsPathSegment_Key _self;
  final $Res Function(YrsPathSegment_Key) _then;

  /// Create a copy of YrsPathSegment
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(YrsPathSegment_Key(
      null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class YrsPathSegment_Index extends YrsPathSegment {
  const YrsPathSegment_Index(this.field0) : super._();

  @override
  final int field0;

  /// Create a copy of YrsPathSegment
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $YrsPathSegment_IndexCopyWith<YrsPathSegment_Index> get copyWith =>
      _$YrsPathSegment_IndexCopyWithImpl<YrsPathSegment_Index>(
          this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is YrsPathSegment_Index &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'YrsPathSegment.index(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $YrsPathSegment_IndexCopyWith<$Res>
    implements $YrsPathSegmentCopyWith<$Res> {
  factory $YrsPathSegment_IndexCopyWith(YrsPathSegment_Index value,
          $Res Function(YrsPathSegment_Index) _then) =
      _$YrsPathSegment_IndexCopyWithImpl;
  @useResult
  $Res call({int field0});
}

/// @nodoc
class _$YrsPathSegment_IndexCopyWithImpl<$Res>
    implements $YrsPathSegment_IndexCopyWith<$Res> {
  _$YrsPathSegment_IndexCopyWithImpl(this._self, this._then);

  final YrsPathSegment_Index _self;
  final $Res Function(YrsPathSegment_Index) _then;

  /// Create a copy of YrsPathSegment
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(YrsPathSegment_Index(
      null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as int,
    ));
  }
}

/// @nodoc
mixin _$YrsTransferLocation {
  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is YrsTransferLocation);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  String toString() {
    return 'YrsTransferLocation()';
  }
}

/// @nodoc
class $YrsTransferLocationCopyWith<$Res> {
  $YrsTransferLocationCopyWith(
      YrsTransferLocation _, $Res Function(YrsTransferLocation) __);
}

/// Adds pattern-matching-related methods to [YrsTransferLocation].
extension YrsTransferLocationPatterns on YrsTransferLocation {
  /// A variant of `map` that fallback to returning `orElse`.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case _:
  ///     return orElse();
  /// }
  /// ```

  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(YrsTransferLocation_Array value)? array,
    TResult Function(YrsTransferLocation_Map value)? map,
    TResult Function(YrsTransferLocation_ArrayAtMapKey value)? arrayAtMapKey,
    required TResult orElse(),
  }) {
    final _that = this;
    switch (_that) {
      case YrsTransferLocation_Array() when array != null:
        return array(_that);
      case YrsTransferLocation_Map() when map != null:
        return map(_that);
      case YrsTransferLocation_ArrayAtMapKey() when arrayAtMapKey != null:
        return arrayAtMapKey(_that);
      case _:
        return orElse();
    }
  }

  /// A `switch`-like method, using callbacks.
  ///
  /// Callbacks receives the raw object, upcasted.
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case final Subclass2 value:
  ///     return ...;
  /// }
  /// ```

  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(YrsTransferLocation_Array value) array,
    required TResult Function(YrsTransferLocation_Map value) map,
    required TResult Function(YrsTransferLocation_ArrayAtMapKey value)
        arrayAtMapKey,
  }) {
    final _that = this;
    switch (_that) {
      case YrsTransferLocation_Array():
        return array(_that);
      case YrsTransferLocation_Map():
        return map(_that);
      case YrsTransferLocation_ArrayAtMapKey():
        return arrayAtMapKey(_that);
    }
  }

  /// A variant of `map` that fallback to returning `null`.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case _:
  ///     return null;
  /// }
  /// ```

  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(YrsTransferLocation_Array value)? array,
    TResult? Function(YrsTransferLocation_Map value)? map,
    TResult? Function(YrsTransferLocation_ArrayAtMapKey value)? arrayAtMapKey,
  }) {
    final _that = this;
    switch (_that) {
      case YrsTransferLocation_Array() when array != null:
        return array(_that);
      case YrsTransferLocation_Map() when map != null:
        return map(_that);
      case YrsTransferLocation_ArrayAtMapKey() when arrayAtMapKey != null:
        return arrayAtMapKey(_that);
      case _:
        return null;
    }
  }

  /// A variant of `when` that fallback to an `orElse` callback.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case _:
  ///     return orElse();
  /// }
  /// ```

  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(List<YrsPathSegment> path, int index)? array,
    TResult Function(List<YrsPathSegment> path, String key)? map,
    TResult Function(List<YrsPathSegment> parentPath, String key, int index)?
        arrayAtMapKey,
    required TResult orElse(),
  }) {
    final _that = this;
    switch (_that) {
      case YrsTransferLocation_Array() when array != null:
        return array(_that.path, _that.index);
      case YrsTransferLocation_Map() when map != null:
        return map(_that.path, _that.key);
      case YrsTransferLocation_ArrayAtMapKey() when arrayAtMapKey != null:
        return arrayAtMapKey(_that.parentPath, _that.key, _that.index);
      case _:
        return orElse();
    }
  }

  /// A `switch`-like method, using callbacks.
  ///
  /// As opposed to `map`, this offers destructuring.
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case Subclass2(:final field2):
  ///     return ...;
  /// }
  /// ```

  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(List<YrsPathSegment> path, int index) array,
    required TResult Function(List<YrsPathSegment> path, String key) map,
    required TResult Function(
            List<YrsPathSegment> parentPath, String key, int index)
        arrayAtMapKey,
  }) {
    final _that = this;
    switch (_that) {
      case YrsTransferLocation_Array():
        return array(_that.path, _that.index);
      case YrsTransferLocation_Map():
        return map(_that.path, _that.key);
      case YrsTransferLocation_ArrayAtMapKey():
        return arrayAtMapKey(_that.parentPath, _that.key, _that.index);
    }
  }

  /// A variant of `when` that fallback to returning `null`
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case _:
  ///     return null;
  /// }
  /// ```

  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(List<YrsPathSegment> path, int index)? array,
    TResult? Function(List<YrsPathSegment> path, String key)? map,
    TResult? Function(List<YrsPathSegment> parentPath, String key, int index)?
        arrayAtMapKey,
  }) {
    final _that = this;
    switch (_that) {
      case YrsTransferLocation_Array() when array != null:
        return array(_that.path, _that.index);
      case YrsTransferLocation_Map() when map != null:
        return map(_that.path, _that.key);
      case YrsTransferLocation_ArrayAtMapKey() when arrayAtMapKey != null:
        return arrayAtMapKey(_that.parentPath, _that.key, _that.index);
      case _:
        return null;
    }
  }
}

/// @nodoc

class YrsTransferLocation_Array extends YrsTransferLocation {
  const YrsTransferLocation_Array(
      {required final List<YrsPathSegment> path, required this.index})
      : _path = path,
        super._();

  final List<YrsPathSegment> _path;
  List<YrsPathSegment> get path {
    if (_path is EqualUnmodifiableListView) return _path;
    // ignore: implicit_dynamic_type
    return EqualUnmodifiableListView(_path);
  }

  final int index;

  /// Create a copy of YrsTransferLocation
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $YrsTransferLocation_ArrayCopyWith<YrsTransferLocation_Array> get copyWith =>
      _$YrsTransferLocation_ArrayCopyWithImpl<YrsTransferLocation_Array>(
          this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is YrsTransferLocation_Array &&
            const DeepCollectionEquality().equals(other._path, _path) &&
            (identical(other.index, index) || other.index == index));
  }

  @override
  int get hashCode => Object.hash(
      runtimeType, const DeepCollectionEquality().hash(_path), index);

  @override
  String toString() {
    return 'YrsTransferLocation.array(path: $path, index: $index)';
  }
}

/// @nodoc
abstract mixin class $YrsTransferLocation_ArrayCopyWith<$Res>
    implements $YrsTransferLocationCopyWith<$Res> {
  factory $YrsTransferLocation_ArrayCopyWith(YrsTransferLocation_Array value,
          $Res Function(YrsTransferLocation_Array) _then) =
      _$YrsTransferLocation_ArrayCopyWithImpl;
  @useResult
  $Res call({List<YrsPathSegment> path, int index});
}

/// @nodoc
class _$YrsTransferLocation_ArrayCopyWithImpl<$Res>
    implements $YrsTransferLocation_ArrayCopyWith<$Res> {
  _$YrsTransferLocation_ArrayCopyWithImpl(this._self, this._then);

  final YrsTransferLocation_Array _self;
  final $Res Function(YrsTransferLocation_Array) _then;

  /// Create a copy of YrsTransferLocation
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? path = null,
    Object? index = null,
  }) {
    return _then(YrsTransferLocation_Array(
      path: null == path
          ? _self._path
          : path // ignore: cast_nullable_to_non_nullable
              as List<YrsPathSegment>,
      index: null == index
          ? _self.index
          : index // ignore: cast_nullable_to_non_nullable
              as int,
    ));
  }
}

/// @nodoc

class YrsTransferLocation_Map extends YrsTransferLocation {
  const YrsTransferLocation_Map(
      {required final List<YrsPathSegment> path, required this.key})
      : _path = path,
        super._();

  final List<YrsPathSegment> _path;
  List<YrsPathSegment> get path {
    if (_path is EqualUnmodifiableListView) return _path;
    // ignore: implicit_dynamic_type
    return EqualUnmodifiableListView(_path);
  }

  final String key;

  /// Create a copy of YrsTransferLocation
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $YrsTransferLocation_MapCopyWith<YrsTransferLocation_Map> get copyWith =>
      _$YrsTransferLocation_MapCopyWithImpl<YrsTransferLocation_Map>(
          this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is YrsTransferLocation_Map &&
            const DeepCollectionEquality().equals(other._path, _path) &&
            (identical(other.key, key) || other.key == key));
  }

  @override
  int get hashCode =>
      Object.hash(runtimeType, const DeepCollectionEquality().hash(_path), key);

  @override
  String toString() {
    return 'YrsTransferLocation.map(path: $path, key: $key)';
  }
}

/// @nodoc
abstract mixin class $YrsTransferLocation_MapCopyWith<$Res>
    implements $YrsTransferLocationCopyWith<$Res> {
  factory $YrsTransferLocation_MapCopyWith(YrsTransferLocation_Map value,
          $Res Function(YrsTransferLocation_Map) _then) =
      _$YrsTransferLocation_MapCopyWithImpl;
  @useResult
  $Res call({List<YrsPathSegment> path, String key});
}

/// @nodoc
class _$YrsTransferLocation_MapCopyWithImpl<$Res>
    implements $YrsTransferLocation_MapCopyWith<$Res> {
  _$YrsTransferLocation_MapCopyWithImpl(this._self, this._then);

  final YrsTransferLocation_Map _self;
  final $Res Function(YrsTransferLocation_Map) _then;

  /// Create a copy of YrsTransferLocation
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? path = null,
    Object? key = null,
  }) {
    return _then(YrsTransferLocation_Map(
      path: null == path
          ? _self._path
          : path // ignore: cast_nullable_to_non_nullable
              as List<YrsPathSegment>,
      key: null == key
          ? _self.key
          : key // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class YrsTransferLocation_ArrayAtMapKey extends YrsTransferLocation {
  const YrsTransferLocation_ArrayAtMapKey(
      {required final List<YrsPathSegment> parentPath,
      required this.key,
      required this.index})
      : _parentPath = parentPath,
        super._();

  final List<YrsPathSegment> _parentPath;
  List<YrsPathSegment> get parentPath {
    if (_parentPath is EqualUnmodifiableListView) return _parentPath;
    // ignore: implicit_dynamic_type
    return EqualUnmodifiableListView(_parentPath);
  }

  final String key;
  final int index;

  /// Create a copy of YrsTransferLocation
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $YrsTransferLocation_ArrayAtMapKeyCopyWith<YrsTransferLocation_ArrayAtMapKey>
      get copyWith => _$YrsTransferLocation_ArrayAtMapKeyCopyWithImpl<
          YrsTransferLocation_ArrayAtMapKey>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is YrsTransferLocation_ArrayAtMapKey &&
            const DeepCollectionEquality()
                .equals(other._parentPath, _parentPath) &&
            (identical(other.key, key) || other.key == key) &&
            (identical(other.index, index) || other.index == index));
  }

  @override
  int get hashCode => Object.hash(runtimeType,
      const DeepCollectionEquality().hash(_parentPath), key, index);

  @override
  String toString() {
    return 'YrsTransferLocation.arrayAtMapKey(parentPath: $parentPath, key: $key, index: $index)';
  }
}

/// @nodoc
abstract mixin class $YrsTransferLocation_ArrayAtMapKeyCopyWith<$Res>
    implements $YrsTransferLocationCopyWith<$Res> {
  factory $YrsTransferLocation_ArrayAtMapKeyCopyWith(
          YrsTransferLocation_ArrayAtMapKey value,
          $Res Function(YrsTransferLocation_ArrayAtMapKey) _then) =
      _$YrsTransferLocation_ArrayAtMapKeyCopyWithImpl;
  @useResult
  $Res call({List<YrsPathSegment> parentPath, String key, int index});
}

/// @nodoc
class _$YrsTransferLocation_ArrayAtMapKeyCopyWithImpl<$Res>
    implements $YrsTransferLocation_ArrayAtMapKeyCopyWith<$Res> {
  _$YrsTransferLocation_ArrayAtMapKeyCopyWithImpl(this._self, this._then);

  final YrsTransferLocation_ArrayAtMapKey _self;
  final $Res Function(YrsTransferLocation_ArrayAtMapKey) _then;

  /// Create a copy of YrsTransferLocation
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? parentPath = null,
    Object? key = null,
    Object? index = null,
  }) {
    return _then(YrsTransferLocation_ArrayAtMapKey(
      parentPath: null == parentPath
          ? _self._parentPath
          : parentPath // ignore: cast_nullable_to_non_nullable
              as List<YrsPathSegment>,
      key: null == key
          ? _self.key
          : key // ignore: cast_nullable_to_non_nullable
              as String,
      index: null == index
          ? _self.index
          : index // ignore: cast_nullable_to_non_nullable
              as int,
    ));
  }
}

// dart format on
